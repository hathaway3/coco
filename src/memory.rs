use crate::config;
use crate::cpu::Core;
use crate::error::Error;
use crate::u8oru16::u8u16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]

pub enum AccessType {
    Program,
    UserStack,
    SystemStack,
    Generic,
    System,
}

impl Core {
    // reads one byte from RAM
    #[inline(always)]
    pub fn _read_u8(&self, _: AccessType, addr: u16, data: Option<&mut u8>) -> Result<u8, Error> {
        if config::debug() {
            self.debug_check_for_watch_hit(addr);
        }
        let byte = if addr < 0xff00 {
            self.raw_ram[addr as usize]
        } else {
            if let Some(acia_rc) = self.acia.as_ref() {
                let mut acia = acia_rc.borrow_mut();
                if acia.owns_address(addr) {
                    let b = acia.read(addr)?;
                    if let Some(data) = data {
                        *data = b;
                    }
                    return Ok(b);
                }
            }
            match addr {
                0xff00..=0xff3f => {
                    let mut pia = self.pia.lock();
                    pia.read(addr)
                }
                0xffc0..=0xffdf => 0u8,
                0xffe0..=0xffff => self.raw_ram[(addr - 0x4000) as usize],
                _ => {
                    warn!("Read at unimplemented address {:04x}", addr);
                    0
                }
            }
        };
        if let Some(data) = data {
            *data = byte;
        }
        Ok(byte)
    }
    // helper version of _read_u8 that reads a byte into a u16
    #[inline(always)]
    pub fn _read_u8_as_u16(
        &self,
        atype: AccessType,
        addr: u16,
        data: Option<&mut u16>,
    ) -> Result<u16, Error> {
        let byte = self._read_u8(atype, addr, None)?;
        let word = byte as u16;
        if let Some(data) = data {
            *data = word
        }
        Ok(word)
    }
    // version of _read... for u16
    // reads two bytes as a u16 (high order byte first)
    #[inline(always)]
    pub fn _read_u16(
        &self,
        atype: AccessType,
        addr: u16,
        data: Option<&mut u16>,
    ) -> Result<u16, Error> {
        let mut b: [u8; 2] = [0, 0];
        self._read_u8(atype, addr, Some(&mut b[0]))?;
        self._read_u8(atype, addr.wrapping_add(1), Some(&mut b[1]))?;
        let word = (b[0] as u16) << 8 | (b[1] as u16);
        if let Some(data) = data {
            *data = word;
        }
        Ok(word)
    }
    // version of _read... for u8u16
    #[inline(always)]
    pub fn _read_u8u16(&self, atype: AccessType, addr: u16, size: u16) -> Result<u8u16, Error> {
        match size {
            1 => {
                let b = self._read_u8(atype, addr, None)?;
                Ok(u8u16::u8(b))
            }
            2 => {
                let w = self._read_u16(atype, addr, None)?;
                Ok(u8u16::u16(w))
            }
            _ => panic!("invalid read size for _read_u8u16"),
        }
    }
    //
    // writes
    //
    #[inline(always)]
    pub fn _write_u8(&mut self, at: AccessType, addr: u16, data: u8) -> Result<(), Error> {
        if config::debug() {
            self.debug_check_for_watch_hit(addr);
        }
        if addr < 0xff00 {
            if addr > self.ram_top && at != AccessType::System {
                return Ok(());
            }
            self.raw_ram[addr as usize] = data;
            if addr < 0x8000 {
                self.vram_dirty = true;
            }
        } else {
            if let Some(acia_rc) = self.acia.as_mut() {
                let mut acia = acia_rc.borrow_mut();
                if acia.owns_address(addr) {
                    return acia.write(addr, data);
                }
            }
            match addr {
                0xff00..=0xff3f => {
                    {
                        let mut pia = self.pia.lock();
                        pia.write(addr, data);
                    }
                    if addr == 0xff01 {
                        self.hsync_enabled = (data & 1) == 1;
                    } else if addr == 0xff03 {
                        self.vsync_enabled = (data & 1) == 1;
                    }
                }
                0xffc0..=0xffdf => {
                    let mut sam = self.sam.lock();
                    sam.write((addr - 0xffc0) as usize);
                }
                0xffe0..=0xffff => {
                    if addr > self.ram_top && at != AccessType::System {
                        return Ok(());
                    }
                    self.raw_ram[(addr - 0x4000) as usize] = data;
                }
                _ => warn!("Write at unimplemented address {:04x}", addr),
            }
        }
        Ok(())
    }
    #[inline(always)]
    pub fn _write_u8u16(&mut self, atype: AccessType, addr: u16, data: u8u16) -> Result<(), Error> {
        let mut offset = 0u16;
        if let Some(msb) = data.msb() {
            self._write_u8(atype, addr, msb)?;
            offset += 1;
        }
        self._write_u8(atype, addr.wrapping_add(offset), data.lsb())
    }
}
