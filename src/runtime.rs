// use core::time::Duration; // Already available via crate::Duration

/// Implements the runtime engine of the simulator.
use crate::{
    cpu::InterruptType,
    instructions::{PPPostByte, TEPostByte},
};

use super::*;
use memory::AccessType;

pub const HSYNC_PERIOD_CYCLES: u64 = 64; // Approx for 1MHz
pub const VSYNC_PERIOD_CYCLES: u64 = 16667; // Approx for 1MHz

impl Core {
    /// Resets the 6809 by clearing the registers and
    /// then loading the program counter from the reset vector
    /// (or using the override value if one has been set)
    pub fn reset(&mut self) -> Result<(), Error> {
        self.reg.reset();
        if let Some(addr) = self.reset_vector {
            self.force_reset_vector(addr)?
        }
        // Note that in the color computer, 0xFFnn addresses are remapped to 0xBFnn
        // so the following read is really getting a u16 from 0xBFFFE
        self.reg.pc = self._read_u16(memory::AccessType::System, 0xfffe, None)?;
        self.program_start = self.reg.pc;
        self.faulted = false;
        Ok(())
    }
    pub fn force_reset_vector(&mut self, addr: u16) -> Result<(), Error> {
        self._write_u8u16(memory::AccessType::System, 0xfffe, u8u16::u16(addr))
    }
    /// Displays current perf information to stdout
    #[allow(dead_code)]
    fn report_perf(&self) {
        /*
        if !config::ARGS.perf {
            return;
        }
        */
        // let total_time_ms = 1; // dummy
        info!(
            "Executed {} instructions; effective clock: {} cycles",
            self.instruction_count, self.clock_cycles,
        );
        /*
        info!("\t{:<10} {:>6} {:>5}", "Phase", "Time", "%");
        info!("\t-----------------------");
        macro_rules! perf_row {
            ($name:expr, $id:expr) => {
                info!("\t{:<10} {:>6.3}", $name, $id.as_secs_f32(),)
            };
        }
        perf_row!("meta", self.meta_time);
        perf_row!("prep", self.prep_time);
        perf_row!("eval", self.eval_time);
        perf_row!("commit", self.commit_time);
        perf_row!("total", Duration::ZERO);
        */
    }
    /// Starts executing instructions at the current program counter.  
    /// Does not set or read any registers before attempting to execute.  
    /// Will attempt to execute until an EXIT psuedo-instruction or an
    /// unhandled exception is encountered.
    pub fn exec(&mut self) -> Result<(), Error> {
        self.start_time = self.clock_cycles;
        loop {
            let temp_pc = self.reg.pc;
            if let Err(e) = self.exec_one() {
                if e.kind == ErrorKind::Exit {
                    // this is a normal exit
                    break;
                }
                // if the debugger is disabled then stop executing and return the error
                // otherwise, the debug cli will be invoked when we try to exec the next instruction (due to the fault)
                if !config::debug() {
                    return Err(e);
                } else {
                    self.fault(temp_pc, &e);
                }
            }
            /*
            if let Some(time) = config::ARGS.time {
                if self.start_time.elapsed() > Duration::from_secs_f32(time) {
                    info!("Terminating because the specified time has expired.");
                    break;
                }
            }
            */
        }
        /*
        if config::ARGS.perf {
            self.report_perf()
        }
        */
        Ok(())
    }
    /// Helper function for exec.  
    /// Wraps calls to exec_next and adds debug checks and interrupt processing.
    pub fn exec_one(&mut self) -> Result<(), Error> {
        let _function_start = self.clock_cycles;
        // let mut meta_start: Option<u64> = None;
        let _expected_duration_cycles: Option<u64> = None;
        if config::debug() && self.pre_instruction_debug_check(self.reg.pc) {
            self.debug_cli()?;
        }
        let temp_pc = self.reg.pc;
        if !self.in_cwai && !self.in_sync {
            let outcome = self.exec_next(self.list_mode.is_none())?;
            // meta_start = Some(self.clock_cycles);
            // if paying attention to timing then track how long this instruction should have taken
            /*
            expected_duration = self
                .min_cycle
                .and_then(|min| min.checked_mul(outcome.inst.flavor.detail.clk as u32));
            */
            // check for meta instructions (interrupts, SYNC, CWAI, EXIT)
            if let Some(meta) = outcome.meta.as_ref() {
                let it = meta.to_interrupt_type();
                match meta {
                    instructions::Meta::EXIT => {
                        info!("EXIT instruction at PC={:0x}", self.reg.pc);
                        return Err(Error::new(
                            ErrorKind::Exit,
                            None,
                            "program terminated by EXIT instruction",
                        ));
                    }
                    instructions::Meta::CWAI => {
                        self.stack_for_interrupt(true)?;
                        self.in_cwai = true;
                        verbose_println!("CWAI at PC={:0x}: waiting for interrupt...", self.reg.pc);
                    }
                    instructions::Meta::SYNC => {
                        self.in_sync = true;
                        verbose_println!("SYNC at PC={:0x}: waiting for interrupt...", self.reg.pc);
                    }
                    _ if it.is_some() => {
                        self.start_interrupt(it.unwrap())?;
                    }
                    _ => {
                        panic!("meta-instruction {:?} not supported", meta);
                    }
                }
            }
            if config::help_humans() {
                self.post_instruction_debug_check(temp_pc, &outcome);
            }
        }
        /*
        if meta_start.is_none() {
            meta_start = Some(self.clock_cycles);
        }
        */
        let mut irq;
        let mut firq = false;
        // check for work that needs to be done on hsync
        // (using hsync as the period at which to poll for pending interrupts
        // rather than checking between every instruction)
        if self.clock_cycles - self.hsync_prev >= HSYNC_PERIOD_CYCLES {
            self.hsync_prev = self.clock_cycles;
            // check for hardware firq
            {
                let mut pia1 = self.pia1.lock();
                if self.cart_pending {
                    firq = pia1.cart_firq();
                }
            }
            // check for hardware irq
            {
                let mut pia0 = self.pia0.lock();
                irq = pia0.hsync_irq();
            }
            // if it's vsync time, then also check for vsync irq
            if self.clock_cycles - self.vsync_prev >= VSYNC_PERIOD_CYCLES {
                self.vsync_prev = self.clock_cycles;
                {
                    let mut pia0 = self.pia0.lock();
                    irq = irq || pia0.vsync_irq();
                }
            }
            if irq {
                // hardware issued an hsync irq
                // sync completes whether or not we service the interrupt
                self.in_sync = false;
                // if irq is not masked then service it
                if !self.reg.cc.is_set(registers::CCBit::I) {
                    self.start_interrupt(InterruptType::Irq)?;
                }
            }
            if firq {
                // hardware issued a firq
                // sync completes whether or not we service the interrupt
                self.in_sync = false;
                // if FIRQ is not masked then service it
                if !self.reg.cc.is_set(registers::CCBit::F) {
                    self.start_interrupt(InterruptType::Firq)?;
                    self.cart_pending = false;
                }
            }
        }
        /*
        if let Some(ms) = meta_start {
             self.meta_time += Duration::from_micros(self.clock_cycles - ms);
        }
        */
        Ok(())
    }

    // helper function for interrupt handling
    // simply pushes the named register on the system stack
    pub fn system_psh(&mut self, reg: registers::Name) -> Result<(), Error> {
        let mut addr = self.reg.get_register(registers::Name::S).u16();
        if addr < registers::reg_size(reg) {
            return Err(runtime_err!(Some(self.reg), "interal_push stack overflow"));
        }
        addr -= registers::reg_size(reg);
        self._write_u8u16(AccessType::System, addr, self.reg.get_register(reg))?;
        self.reg.set_register(registers::Name::S, u8u16::u16(addr));
        Ok(())
    }
    // sets up the stack frame for an interrupt
    pub fn stack_for_interrupt(&mut self, entire: bool) -> Result<(), Error> {
        // save the appropriate registers
        self.system_psh(registers::Name::PC)?;
        if entire {
            self.system_psh(registers::Name::U)?;
            self.system_psh(registers::Name::Y)?;
            self.system_psh(registers::Name::X)?;
            self.system_psh(registers::Name::DP)?;
            self.system_psh(registers::Name::B)?;
            self.system_psh(registers::Name::A)?;
        }
        // remember whether we pushed everything onto the stack
        // Note that this flag is set in cc prior to pushing cc on the stack
        self.reg.cc.set(registers::CCBit::E, entire);
        self.system_psh(registers::Name::CC)?;
        Ok(())
    }
    /// Sets the CC register and stack as appropriate and
    /// then sets PC to the vector for the given interrupt.
    pub fn start_interrupt(&mut self, it: crate::cpu::InterruptType) -> Result<(), Error> {
        assert!(!self.in_sync);
        // info!("start_interrupt {:?}, vector {:04x}", it, it.vector());
        // if this is an IRQ then we need to push (almost) everything on the stack
        let mut entire = false;
        use crate::cpu::InterruptType::*;
        let mut if_mask_flags: u8 = 0;
        match it {
            Swi2 | Swi3 => {
                entire = true;
            }
            Irq => {
                entire = true;
                if_mask_flags = 0x10;
            }
            Firq => {
                if_mask_flags = 0x50;
            }
            _ => {
                entire = true;
                if_mask_flags = 0x50;
            }
        }
        // save current state prior to interrupt
        // but only if we aren't already waiting for an interrupt
        // (because if we are, then the state was already saved)
        if !self.in_cwai {
            self.stack_for_interrupt(entire)?;
        }
        // now set the appropriate flags in CC
        self.reg.cc.or_with_byte(if_mask_flags);
        // get the vector for the ISR
        let addr = self._read_u16(AccessType::System, it.vector(), None)?;
        // check to see if the vector points to a zero byte; if so then panic
        let b = self._read_u8(AccessType::System, addr, None)?;
        if b == 0 {
            panic!("interrupt {:?} vector points to zero instruction", it)
        }
        // set the program counter
        self.reg.set_register(registers::Name::PC, u8u16::u16(addr));
        // we're no longer waiting for an interrupt
        self.in_cwai = false;
        Ok(())
    }
    /// Attempt to execute the next instruction at PC.  
    /// If commit=true then commit any/all changes to the machine state.
    /// Otherwise, the changes are only reflected in the instruction::Outcome object.
    /// If list_mode.is_some() then the instruction is not evaluated and Outcome reflects
    /// the state prior to the instruction.
    pub fn exec_next(&mut self, _commit: bool) -> Result<instructions::Outcome, Error> {
        // let mut start = Instant::now();
        let mut inst = instructions::Instance::new(self.reg.pc, None);
        let mut op16: u16 = 0; // 16-bit representation of the opcode

        // get the base op code
        loop {
            inst.buf[inst.size as usize] =
                self._read_u8(AccessType::Program, self.reg.pc + inst.size, None)?;
            op16 |= inst.buf[inst.size as usize] as u16;
            inst.size += 1;
            if inst.size == 1 && instructions::is_high_byte_of_16bit_instruction(inst.buf[0]) {
                op16 <<= 8;
                continue;
            }
            break;
        }
        // keep track of how many bytes the opcode takes up
        inst.opsize = inst.size;
        // get the instruction Flavor
        // Note: doing this with if/else rather than ok_or or ok_or_else because it performs better
        inst.flavor = if let Some(flavor) = instructions::opcode_to_flavor(op16) {
            flavor
        } else {
            return Err(runtime_err!(
                Some(self.reg),
                "Bad instruction: {:04X} found at {:04X}",
                op16,
                self.reg.pc
            ));
        };
        self.process_addressing_mode(&mut inst)?;

        assert!(inst.size >= inst.flavor.detail.sz);
        // adjust the program counter before evaluating instructions
        self.reg.pc = self.checked_pc_add(self.reg.pc, inst.size, &inst)?;
        let mut o = instructions::Outcome::new(inst);
        // track how long all this preparation took
        // self.prep_time += start.elapsed();
        // start = Instant::now();

        // evaluate the instruction if we're not in list mode
        if self.list_mode.is_none() {
            (o.inst.flavor.desc.eval)(self, &mut o)?;
        }
        // self.eval_time += start.elapsed();
        // start = Instant::now();

        // if caller wants to commit the changes and we're not in list mode then commit now

        // self.commit_time += start.elapsed();

        self.instruction_count += 1;
        self.clock_cycles += o.inst.flavor.detail.clk as u64;
        Ok(o)
    }
    /// Increase the program counter by the given value (rhs).
    /// Returns Error::Runtime in the case of overflow.
    /// Otherwise, Ok.
    #[inline(always)]
    fn checked_pc_add(
        &self,
        pc: u16,
        rhs: u16,
        _inst: &instructions::Instance,
    ) -> Result<u16, Error> {
        // avoiding ok_or and ok_or_else to increase performance
        // ok_or would invoke the runtime_err! macro every time (regardless of result)
        // ok_or_else seems to be slightly slower than manually checking with if/else
        if let Some(pc) = pc.checked_add(rhs) {
            Ok(pc)
        } else {
            Err(runtime_err!(
                Some(self.reg),
                "Instruction overflow: instruction {} at {:04X}",
                inst.flavor.desc.name,
                self.reg.pc
            ))
        }
    }

    /// Determine the effective address for the instruction, update the instruction size,
    /// modify any registers that are changed by the addressing mode (e.g. ,X+),
    /// and provide a disassembled string representing the operand (if help_humans() == true).
    /// Changes are reflected in the provided inst and self.reg objects.
    fn process_addressing_mode(&mut self, inst: &mut instructions::Instance) -> Result<(), Error> {
        match inst.flavor.mode {
            instructions::AddressingMode::Immediate => {
                // effective address is the current PC
                inst.ea = self.checked_pc_add(self.reg.pc, inst.size, inst)?;
                let addr_size = inst.flavor.detail.sz - inst.size;
                let data = self._read_u8u16(AccessType::Program, inst.ea, addr_size)?;
                inst.size += addr_size;
                if config::help_humans() {
                    inst.operand = Some(match inst.flavor.desc.pbt {
                        instructions::PBT::NA => format!("#${}", data),
                        instructions::PBT::TransferExchange => TEPostByte::to_string(data.u8()),
                        instructions::PBT::PushPull => PPPostByte::to_string(
                            data.u8(),
                            inst.flavor.desc.reg == registers::Name::U,
                        ),
                    });
                }
            }
            instructions::AddressingMode::Direct => {
                // effective address is u16 whose high byte = DP
                // and low byte is stored at the current PC
                inst.ea = ((self.reg.dp as u16) << 8)
                    | (self._read_u8(
                        AccessType::Program,
                        self.checked_pc_add(self.reg.pc, inst.size, inst)?,
                        None,
                    )? as u16);
                inst.size += 1;
                if config::help_humans() {
                    inst.operand = Some(format!("${:04X}", inst.ea));
                }
            }
            instructions::AddressingMode::Extended => {
                // effective address is u16 stored at current PC
                inst.ea = self._read_u16(
                    AccessType::Program,
                    self.checked_pc_add(self.reg.pc, inst.size, inst)?,
                    None,
                )?;
                inst.size += 2;
                if config::help_humans() {
                    inst.operand = Some(format!("${:04X}", inst.ea));
                }
            }
            instructions::AddressingMode::Inherent => {
                // nothing to do. op code itself is sufficient
            }
            instructions::AddressingMode::Relative => {
                let offset_size = inst.flavor.detail.sz - inst.size;
                let offset = self._read_u8u16(
                    AccessType::Program,
                    self.checked_pc_add(self.reg.pc, inst.size, inst)?,
                    offset_size,
                )?;
                inst.size += offset_size;
                inst.ea = u8u16::u16(self.checked_pc_add(self.reg.pc, inst.size, inst)?)
                    .signed_offset(offset)
                    .u16();
                if config::help_humans() {
                    inst.operand = Some(format!("{} ({:04x})", offset.i16(), inst.ea));
                }
            }
            instructions::AddressingMode::Indexed => {
                // todo: move this to a function?
                // read the post-byte
                let pb = self._read_u8(
                    AccessType::Program,
                    self.checked_pc_add(self.reg.pc, inst.size, inst)?,
                    None,
                )?;
                inst.size += 1;
                // is this indirect mode?
                let indirect = (pb & 0b10010000) == 0b10010000;
                // note which register (preg) the register field (rr) is referencing
                let rr = (pb & 0b01100000) >> 5;
                let reg_name = match rr {
                    0 => registers::Name::X,
                    1 => registers::Name::Y,
                    2 => registers::Name::U,
                    3 => registers::Name::S,
                    _ => unreachable!(),
                };
                let ir_str = match reg_name {
                    registers::Name::X => "X",
                    registers::Name::Y => "Y",
                    registers::Name::U => "U",
                    registers::Name::S => "S",
                    _ => "",
                };
                let mut ir_val = self.reg.get_register(reg_name).u16();
                match pb & 0x8f {
                    0..=0b11111 => {
                        // ,R + 5 bit offset
                        let offset =
                            ((pb & 0b11111) | if pb & 0b10000 != 0 { 0b11100000 } else { 0 }) as i8;
                        let (addr, _) = u16::overflowing_add(ir_val, offset as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("{},{}", offset, ir_str))
                        }
                    }
                    0b10000000 => {
                        // ,R+
                        if indirect {
                            return Err(Error::new(
                                ErrorKind::Syntax,
                                Some(self.reg),
                                format!(
                                    "Illegal indirect indexed addressing mode [,R+] at {:04X}",
                                    self.reg.pc
                                )
                                .as_str(),
                            ));
                        }
                        inst.ea = ir_val;
                        let (r, _) = (ir_val).overflowing_add(1);
                        ir_val = r; self.reg.set_register(reg_name, u8u16::u16(ir_val));
                        if config::help_humans() {
                            inst.operand = Some(format!(",{}+", ir_str));
                        }
                    }
                    0b10000001 => {
                        // ,R++
                        inst.ea = ir_val;
                        let (r, _) = (ir_val).overflowing_add(2);
                        ir_val = r; self.reg.set_register(reg_name, u8u16::u16(ir_val));
                        if config::help_humans() {
                            inst.operand = Some(format!(",{}++", ir_str));
                        }
                    }
                    0b10000010 => {
                        // ,-R
                        if indirect {
                            return Err(Error::new(
                                ErrorKind::Syntax,
                                Some(self.reg),
                                format!(
                                    "Illegal indirect indexed addressing mode [,-R] at {:04X}",
                                    self.reg.pc
                                )
                                .as_str(),
                            ));
                        }
                        let (r, _) = (ir_val).overflowing_sub(1);
                        ir_val = r; self.reg.set_register(reg_name, u8u16::u16(ir_val));
                        inst.ea = ir_val;
                        if config::help_humans() {
                            inst.operand = Some(format!(",-{}", ir_str));
                        }
                    }
                    0b10000011 => {
                        // ,--R
                        let (r, _) = (ir_val).overflowing_sub(2);
                        ir_val = r; self.reg.set_register(reg_name, u8u16::u16(ir_val));
                        inst.ea = ir_val;
                        if config::help_humans() {
                            inst.operand = Some(format!(",--{}", ir_str));
                        }
                    }
                    0b10000100 => {
                        // EA = ,R + 0 offset
                        inst.ea = ir_val;
                        if config::help_humans() {
                            inst.operand = Some(format!(",{}", ir_str));
                        }
                    }
                    0b10000101 => {
                        // EA = ,R + B offset
                        let (addr, _) = u16::overflowing_add(ir_val, (self.reg.b as i8) as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("B,{}", ir_str));
                        }
                    }
                    0b10000110 => {
                        // EA = ,R + A offset
                        let (addr, _) = u16::overflowing_add(ir_val, (self.reg.a as i8) as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("A,{}", ir_str));
                        }
                    }
                    // 0b10000111 => {} invalid
                    0b10001000 => {
                        // EA = ,R + 8 bit offset
                        let offset =
                            self._read_u8(AccessType::Program, self.reg.pc + inst.size, None)?
                                as i8;
                        inst.size += 1;
                        let (addr, _) = u16::overflowing_add(ir_val, offset as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("{},{}", offset, ir_str));
                        }
                    }
                    0b10001001 => {
                        // ,R + 16 bit offset
                        let offset =
                            self._read_u16(AccessType::Program, self.reg.pc + inst.size, None)?
                                as i16;
                        inst.size += 2;
                        let (addr, _) = u16::overflowing_add(ir_val, offset as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("{},{}", offset, ir_str));
                        }
                    }
                    // 0b10001010 => {} invalid
                    0b10001011 => {
                        // ,R + D offset
                        let (addr, _) = u16::overflowing_add(ir_val, self.reg.d);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("D,{}", ir_str));
                        }
                    }
                    0b10001100 => {
                        // ,PC + 8 bit offset
                        let offset =
                            self._read_u8(AccessType::Program, self.reg.pc + inst.size, None)?
                                as i8;
                        inst.size += 1;
                        // Note: effective address is relative to the program counter's NEW value (the address of the next instruction)
                        let (pc, _) = u16::overflowing_add(self.reg.pc, inst.size);
                        let (addr, _) = u16::overflowing_add(pc, offset as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("{},PC", offset));
                        }
                    }
                    0b10001101 => {
                        // ,PC + 16 bit offset
                        let offset =
                            self._read_u16(AccessType::Program, self.reg.pc + inst.size, None)?
                                as i16;
                        inst.size += 2;
                        // Note: effective address is relative to the program counter's NEW value (the address of the next instruction)
                        let (pc, _) = u16::overflowing_add(self.reg.pc, inst.size);
                        let (addr, _) = u16::overflowing_add(pc, offset as u16);
                        inst.ea = addr;
                        if config::help_humans() {
                            inst.operand = Some(format!("{},PC", offset));
                        }
                    }
                    0b10001111 => {
                        // EA = [,address]
                        inst.ea =
                            self._read_u16(AccessType::Program, self.reg.pc + inst.size, None)?;
                        if config::help_humans() {
                            inst.operand = Some(format!("[{:04X}]", inst.ea));
                        }
                        inst.size += 2;
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Syntax,
                            Some(self.reg),
                            format!(
                                "Invalid indexed addressing post-byte {:02X} in instruction at {:04X}",
                                pb, self.reg.pc
                            )
                            .as_str(),
                        ));
                    }
                }
                // if indirect flag is set then set inst.ea to self.ram[inst.ea]
                if indirect {
                    inst.ea = self._read_u16(AccessType::Generic, inst.ea, None)?;
                }
            }
            _ => panic!("Invalid addressing mode! {:?}", inst.flavor.mode),
        }
        Ok(())
    }
}
