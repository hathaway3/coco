use super::*;

// status register bits
const RDRF: u8 = 0b00000001; // receive data register full
const TDRE: u8 = 0b00000010; // transmit data register empty

pub struct Acia {
    pub addr: u16,
    recv_cache: Option<u8>,
    tx_cache: Option<u8>,
    pub accessed: bool,
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
    pub fn write(&mut self, addr: u16, byte: u8) -> Result<(), Error> {
        self.accessed = true;
        if addr == self.control_register_address() {
            // ignore control register writes
            return Ok(());
        } else if addr == self.data_register_address() {
            self.tx_cache = Some(byte);
        }
        Ok(())
    }
    pub fn read(&mut self, addr: u16) -> Result<u8, Error> {
        self.accessed = true;
        let mut flags = 0u8;
        if addr == self.status_register_address() {
            // if there is some data ready to read then set the RDRF bit
            if let Some(_pending_data) = self.recv_cache {
                flags |= RDRF;
            }
            // if tx_cache is empty, set TDRE
            if self.tx_cache.is_none() {
                flags |= TDRE;
            }
            Ok(flags)
        } else if addr == self.data_register_address() {
            // try to get a byte from our cache
            if let Some(byte) = self.recv_cache.take() {
                Ok(byte)
            } else {
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
            tx_cache: None,
            accessed: false,
            tty_count,
        })
    }

    pub fn rx_byte(&mut self, byte: u8) {
        self.recv_cache = Some(byte);
    }

    pub fn take_tx_byte(&mut self) -> Option<u8> {
        self.tx_cache.take()
    }

    pub fn has_rx_space(&self) -> bool {
        self.recv_cache.is_none()
    }
}
