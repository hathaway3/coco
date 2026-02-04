use core::cell::{Cell, RefCell};
// // use super::{assembler::Assembler, test::TestCriterion, *}; // Assembler uses std::fs, might need to cfg out
#[cfg(not(target_os = "none"))]
use super::test::TestCriterion;
use crate::hex::{HexRecordCollection, HexRecordType};
use crate::{acia, config, debug, instructions, pia, sam, vdg, Program};
use crate::{Arc, BTreeMap, Duration, Error, ErrorKind, Mutex, String, Vec, VecDeque};
#[allow(unused)]
#[derive(Debug, PartialEq, Eq)]
pub enum InterruptType {
    Reset,
    Nmi,
    Firq,
    Irq,
    Swi,
    Swi2,
    Swi3,
}
impl InterruptType {
    pub fn vector(&self) -> u16 {
        use InterruptType::*;
        match self {
            Reset => 0xfffe,
            Nmi => 0xfffc,
            Swi => 0xfffa,
            Irq => 0xfff8,
            Firq => 0xfff6,
            Swi2 => 0xfff4,
            Swi3 => 0xfff2,
        }
    }
}
/// The Core struct implements the 6809 processor and debugger.
/// Its implementation spans multiple files: runtime.rs, debug.rs, memory.rs, registers.rs
pub struct Core {
    pub raw_ram: &'static mut [u8], // but the CPU will directly access memory via this slice
    pub ram_top: u16,               // keep track of where the caller wants ram to end
    pub sam: Arc<Mutex<crate::sam::Sam>>,
    pub _vdg: Arc<Mutex<crate::vdg::Vdg>>,
    pub pia0: Arc<Mutex<crate::pia::Pia0>>,
    pub pia1: Arc<Mutex<crate::pia::Pia1>>,
    pub reg: crate::registers::Set, // the full set of 6809 registers
    pub acia: Option<crate::acia::Acia>, // ACIA simulator
    pub reset_vector: Option<u16>,  // overrides the reset vector if set
    /* interrupt processing */
    pub cart_pending: bool, // true if cart is loaded but hasn't been run yet
    pub in_cwai: bool,      // if true, the processor is within a CWAI instruction
    pub in_sync: bool,      // if true, the processor is within a SYNC instruction
    pub hsync_prev: u64,    // microseconds or cycles
    pub vsync_prev: u64,
    /* perf measurement */
    pub start_time: u64,
    pub instruction_count: u64, // the number of instructions executed since the most recent program started
    pub clock_cycles: u64, // the number of clock cycles consumed since the most recent program started
    pub eval_time: Duration, // the total time spent in the eval method of instructions
    pub prep_time: Duration, // the total time spent preparing to call eval methods for all instructions
    pub commit_time: Duration, // the total time spent committing the Outcome of all instructions
    pub meta_time: Duration, // the time spent outside of instruction prep and evaluation
    pub _read_time: Cell<Duration>, // the time spent reading memory (in Cell for interior mutability)
    pub _write_time: Duration,      // the time spent writing to memory
    pub min_cycle: Option<Duration>, // the minimum duration of a clock cycle
    /* fields for debugging */
    pub in_debugger: bool,
    pub breakpoints: Vec<debug::Breakpoint>, // all current breakpoints
    pub watch_hits: RefCell<Vec<u16>>, // tracks writes to addresses for which watch breakpoints have been set
    pub addr_to_sym: BTreeMap<u16, Vec<String>>, // map from address to symbol
    pub sym_to_addr: BTreeMap<String, u16>, // map from symbol to address
    pub list_mode: Option<debug::ListMode>, // equals Some(ListMode) if currently in list (disassemble) mode
    pub program_start: u16, // the starting address of the program; should be equal to reset vector
    pub faulted: bool,      // true if the CPU has faulted (e.g., stack oveflow)
    pub history: Option<VecDeque<String>>, // list of instructions that have been recently executed
    pub step_mode: debug::StepMode, // determines current step mode (see debug.rs)
    pub next_linear_step: u16, // tracks the address of the next contiguous instruction (differs from PC when there is a branch or jump)
    pub trace: bool,           // if true then display each instruction as it's executed
}
impl Core {
    pub fn new(
        ram: &'static mut [u8],
        sam: Arc<Mutex<sam::Sam>>,
        vdg: Arc<Mutex<vdg::Vdg>>,
        pia0: Arc<Mutex<pia::Pia0>>,
        pia1: Arc<Mutex<pia::Pia1>>,
        ram_top: u16,
        acia_addr: Option<u16>,
    ) -> Core {
        instructions::init();
        // The CPU needs fast (non-blocking) access to RAM so we turn the provided memory into a slice
        // that can be directly accessed (without wrappers and locks).
        // SAFETY: This is safe because only the CPU ever writes to RAM and the CPU's reads and writes to
        // RAM all occur on a single thread. The video rendering thread also reads from RAM but it's okay
        // if those reads happen during CPU writes. Worst case outcome is a temporary glitch in the display.
        Core {
            raw_ram: ram,
            ram_top,
            sam,
            _vdg: vdg,
            pia0,
            pia1,
            reg: { Default::default() },
            acia: acia_addr.map(|a| acia::Acia::new(a).expect("failed to start ACIA")),
            reset_vector: None,
            cart_pending: false,
            in_cwai: false,
            in_sync: false,
            hsync_prev: 0,
            vsync_prev: 0,
            start_time: 0,
            instruction_count: 0,
            clock_cycles: 0,
            eval_time: Duration::ZERO,
            prep_time: Duration::ZERO,
            commit_time: Duration::ZERO,
            meta_time: Duration::ZERO,
            _read_time: Cell::new(Duration::ZERO),
            _write_time: Duration::ZERO,
            /*
            min_cycle: unsafe {
                config::ARGS
                    .mhz
                    .map(|m| Duration::from_secs_f32(0.9 / (m * 1e6)))
            },
            */
            min_cycle: None,
            in_debugger: false,
            breakpoints: Vec::new(),
            watch_hits: RefCell::new(Vec::new()),
            addr_to_sym: BTreeMap::new(),
            sym_to_addr: BTreeMap::new(),
            list_mode: None,
            program_start: 0,
            faulted: false,
            history: None,
            step_mode: debug::StepMode::Off,
            next_linear_step: 0,
            trace: unsafe { config::ARGS.trace },
        }
    }

    /*
    pub fn load_program_from_file(&mut self, path: &Path) -> Result<(), Error> {
        // ...
    }
    */
    /// copies the contents of a HexRecordCollection into simulator memory
    pub fn load_hex(&mut self, hex: &HexRecordCollection) -> Result<u16, Error> {
        let mut extent = 0u16;
        let mut eof = false;
        let mut rom_write = false;
        for r in hex.iter() {
            match r.record_type {
                HexRecordType::Data => {
                    if let Some(data) = r.data.as_ref() {
                        if r.address as usize + r.data_size as usize > self.raw_ram.len() {
                            return Err(Error::new(
                                ErrorKind::Memory,
                                None,
                                format!(
                                    "program overflowed system RAM ({} byte object at {:04X})",
                                    r.data_size, r.address
                                )
                                .as_str(),
                            ));
                        }
                        let mut addr = r.address as usize;
                        for &b in data {
                            self.raw_ram[addr] = b;
                            addr += 1;
                            extent += 1;
                            if addr >= self.ram_top as usize {
                                rom_write = true;
                            }
                        }
                    }
                }
                HexRecordType::EndOfFile => {
                    eof = true;
                    break;
                }
                _ => warn!(
                    "ignoring unsupported record type ({}) in hex file.",
                    r.record_type
                ),
            }
        }
        if !eof {
            return Err(general_err!("failed to find EOF record in hex file"));
        }
        if rom_write {
            info!("Portions of this program reside in ROM")
        }
        verbose_println!("loaded {} bytes from hex file", extent);
        /*
        if config::auto_load_syms() {
            if let Some(path) = hex_path {
                match self.try_auto_load_symbols(path) {
                    Ok(n) => info!("Auto-loaded {} symbols.", n),
                    Err(e) => warn!("Failed to auto-load symbols: {}", e),
                }
            }
        }
        */
        Ok(extent)
    }

    pub fn load_bytes(&mut self, bytes: &[u8], addr: u16) -> Result<usize, Error> {
        let mut addr = addr as usize;
        let mut loaded = 0;
        for &b in bytes {
            if addr >= self.raw_ram.len() {
                break;
            }
            self.raw_ram[addr] = b;
            addr += 1;
            loaded += 1;
        }
        Ok(loaded)
    }

    /*
    pub fn load_bin(&mut self, bin_path: &Path, addr: u16) -> Result<usize, Error> {
        // ...
    }
    */

    /*
    pub fn load_cart(&mut self, cart_path: &Path) -> Result<usize, Error> {
        // ...
    }
    */

    pub fn load_program(&mut self, program: &Program) -> Result<u16, Error> {
        let mut extent = 0u16;
        let mut rom_write = false;
        // clean out the reset vector in case it was set by a previous program
        self.force_reset_vector(0)?;
        for line in &program.lines {
            if let Some(bob) = line.obj.as_ref().and_then(|o| o.bob_ref()) {
                if bob.size as usize + bob.addr as usize > self.raw_ram.len() {
                    return Err(Error::new(
                        ErrorKind::Memory,
                        None,
                        format!(
                            "program overflowed system RAM ({} byte object at {:04X})",
                            bob.size, bob.addr
                        )
                        .as_str(),
                    ));
                }
                extent += bob.to_bytes(&mut self.raw_ram[bob.addr as usize..]);
                if bob.addr as usize + bob.size as usize >= self.ram_top as usize {
                    rom_write = true;
                }
            }
        }
        if rom_write {
            info!("Portions of this program reside in ROM")
        }
        verbose_println!("loaded {} bytes", extent);
        /*
        if config::auto_load_syms() {
            if let Some(path) = program_path {
                match self.try_auto_load_symbols(path) {
                    Ok(n) => info!("Auto-loaded {} symbols.", n),
                    Err(e) => warn!("Failed to auto-load symbols: {}", e),
                }
            }
        }
        */
        Ok(extent)
    }
    #[allow(unused)]
    #[cfg(not(target_os = "none"))]
    pub fn check_criteria(&self, criteria: &Vec<TestCriterion>) -> Result<(), Error> {
        Ok(())
    }
}
