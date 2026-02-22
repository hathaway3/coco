use super::*;

// status register bits
const RDRF: u8 = 0b00000001; // receive data register full
const TDRE: u8 = 0b00000010; // transmit data register empty

pub struct Acia {
    pub addr: u16,
    recv_cache: Option<u8>,
    tty_count: Arc<Mutex<i32>>,
}

impl Acia {
    pub fn control_register_address(&self) -> u16 {
        self.addr
    }
    pub fn status_register_address(&self) -> u16 {
        self.addr
    }
    pub fn data_register_address(&self) -> u16 {
        self.addr + 1
    }
    pub fn owns_address(&self, addr: u16) -> bool {
        addr == self.addr || addr == (self.addr + 1)
    }
    pub fn write(&mut self, addr: u16, _byte: u8) -> Result<(), Error> {
        if addr == self.control_register_address() {
            // ignore control register writes
            return Ok(());
        } else if addr == self.data_register_address() {
            // ignore transmit data for now
        }
        Ok(())
    }
    pub fn read(&mut self, addr: u16) -> Result<u8, Error> {
        let mut flags = 0u8;
        if addr == self.status_register_address() {
            // if there is some data ready to read then set the RDRF bit
            if let Some(_pending_data) = self.recv_cache {
                acia_dbg!("ACIA status - pending data {:02X}", pending_data);
                flags |= RDRF;
            }
            // if we have a TTY connected then set the TDRE flag
            let ttyc = self.tty_count.lock();
            if *ttyc > 0 {
                flags |= TDRE;
            }
            Ok(flags)
        } else if addr == self.data_register_address() {
            // try to get a byte from our cache
            if let Some(byte) = self.recv_cache.take() {
                acia_dbg!("ACIA read {:02X}", byte);
                Ok(byte)
            } else {
                // user read the data register when there was no data available.
                // result is undefined? just return a 0?
                Ok(0)
            }
        } else {
            panic!("invalid ACIA read address")
        }
    }
}

impl Acia {
    pub fn new(addr: u16) -> Result<Acia, Error> {
        let tty_count = Arc::new(Mutex::new(0));

        Ok(Acia {
            addr,
            recv_cache: None,
            tty_count,
        })
    }
}
