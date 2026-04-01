use super::*;
use alloc::collections::VecDeque;
use spin::Mutex;

pub trait Pia {
    fn read(&mut self, reg_num: usize) -> u8;
    fn write(&mut self, reg_num: usize, data: u8);
}

/// Implements one "side" of a PIA chip (e.g., MC6821 A or B side)
#[derive(Debug, Default)]
struct PiaSide {
    // control register
    cr: u8,
    // peripheral register
    ir: u8,
    // output register
    or: u8,
    // data direction register
    ddr: u8,
    // control lines
    c1: bool,
    c2: bool,
}

impl PiaSide {
    fn manual_c2_trigger(&self) -> bool {
        self.cr & 0x30 == 0x30
    }
    fn write_control(&mut self, b: u8) {
        // bits 6 & 7 are read-only
        self.cr = (b & 0x3f) | (self.cr & 0xc0);
        // check for setting c2 (when it is configured as an output)
        if self.manual_c2_trigger() {
            // bit 3 controls c2
            self.set_c2(self.cr & 8 == 8);
        }
    }
    fn read_control(&mut self) -> u8 {
        let b = self.cr;
        // control line flags are cleared on read
        self.cr &= 0x3f;
        b
    }
    fn pr_selected(&self) -> bool {
        self.cr & 4 == 4
    }
    fn write(&mut self, index: usize, b: u8) {
        if index & 1 == 1 {
            self.write_control(b)
        } else {
            self.write_data(b)
        }
    }
    fn read(&mut self, index: usize) -> u8 {
        if index & 1 == 1 {
            self.read_control()
        } else {
            self.read_data()
        }
    }
    fn write_data(&mut self, b: u8) {
        // bit 2 in CR determines which register receives the write
        if self.pr_selected() {
            // write to peripheral register
            self.or = b & self.ddr;
        } else {
            // write to DDR
            self.ddr = b;
        }
    }
    fn read_data(&self) -> u8 {
        if self.pr_selected() {
            // bits marked as outputs source from output register
            // bits marked as inputs source from the peripheral register
            self.or & self.ddr | self.ir & !self.ddr
        } else {
            // configured for data register == data direction register
            // just return the contents of DDR
            self.ddr
        }
    }
    fn read_output(&self) -> u8 {
        // ddr controls which bits of the output register are seen
        (self.or & self.ddr) | !self.ddr
    }
    fn set_c1(&mut self, c1: bool) {
        // Note! only supporting low-high transitions
        if c1 && !self.c1 {
            // set c1 flag; (bit 7)
            self.cr |= 0x80;
        }
        self.c1 = c1;
    }
    fn set_c2(&mut self, c2: bool) {
        // Note only supporting low-high transitions
        if c2 && !self.c2 && !self.manual_c2_trigger() {
            // set c2 flag; (bit 6)
            self.cr |= 0x40;
        }
        self.c2 = c2;
    }
    // returns true if an interrupt signal is active
    // and resets the interrupt to inactive
    fn consume_interrupt(&mut self) -> bool {
        let mut interrupt = false;
        // if control line 1 transitioned and interrupt from c1 is enabled in cr...
        if (self.cr & 0x80 == 0x80) && (self.cr & 1 == 1) {
            interrupt = true;
            self.cr &= 0x7f; // reset flag
        }
        // if control line 2 transitioned and interrupt from c2 is enabled in cr...
        // AND control line 2 is configured as an input in cr...
        if (self.cr & 0x40 == 0x40) && (self.cr & 0x28 == 0x8) {
            interrupt = true;
            self.cr &= 0xbf; // reset flag
        }
        interrupt
    }
}

/// Internal chip state for PIA 0 (Keyboard, Joystick)
#[derive(Debug, Default)]
struct Chip0 {
    ab: [PiaSide; 2],
    col: [u8; 8],
    joy_x: u8,
    joy_y: u8,
    joy_sw_1: bool,
    joy_sw_2: bool,
}

impl Chip0 {
    fn read(&mut self, reg_num: usize, dac_val: u8) -> u8 {
        let i = reg_num % 4;
        if i == 0 {
            // caller is reading pia0.a data
            let joy_val = match self.ab[0].c2 {
                false => self.joy_x,
                true => self.joy_y,
            };
            if dac_val > joy_val {
                self.ab[0].ir &= 0x7f; // clear comparator flag
            } else {
                self.ab[0].ir |= 0x80; // set comparator flag
            }
        }
        self.ab[(i >> 1) & 1].read(reg_num)
    }

    fn write(&mut self, reg_num: usize, data: u8) {
        let i = reg_num % 4;
        self.ab[(i >> 1) & 1].write(i, data);
        if i == 2 {
            self.strobe_keyboard();
        }
    }

    fn strobe_keyboard(&mut self) {
        let mut com = 0u8;
        let mut cols = !self.ab[1].read_output();
        if cols != 0 {
            for i in 0..8 {
                if cols & 1 == 1 {
                    com |= self.col[i];
                }
                cols >>= 1;
            }
        }
        if self.joy_sw_1 {
            com |= 0x3 & !cols;
        }
        if self.joy_sw_2 {
            com |= 0xc & !cols;
        }
        self.ab[0].ir = !com;
    }
}

/// Internal chip state for PIA 1 (Video Mode, Sound, Cartridge)
#[derive(Debug)]
struct Chip1 {
    ab: [PiaSide; 2],
    sound_enabled: bool,
    dac_sel_a: bool,
    dac_sel_b: bool,
    last_bit_sound: bool,
}

impl Default for Chip1 {
    fn default() -> Self {
        Self {
            ab: [PiaSide::default(), PiaSide::default()],
            sound_enabled: false,
            dac_sel_a: false,
            dac_sel_b: false,
            last_bit_sound: false,
        }
    }
}

/// Unified PIA implementation for the CoCo
#[derive(Debug, Default)]
pub struct PiaPair {
    chip0: Chip0,
    chip1: Chip1,
    audio_buffer: VecDeque<f32>,
}

impl PiaPair {
    pub fn new() -> Self {
        Self {
            chip0: Chip0::default(),
            chip1: Chip1::default(),
            audio_buffer: VecDeque::with_capacity(1024),
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0xff00..=0xff1f => {
                let dac_val = self.chip1.ab[0].read_output() >> 2;
                self.chip0.read((addr - 0xff00) as usize, dac_val)
            }
            0xff20..=0xff3f => self.chip1.ab[((addr - 0xff20) >> 1) as usize & 1].read((addr - 0xff20) as usize),
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0xff00..=0xff1f => {
                let i = (addr - 0xff00) as usize % 4;
                self.chip0.write(i, data);
                if i == 1 || i == 3 {
                    self.chip1.dac_sel_a = self.chip0.ab[0].c2;
                    self.chip1.dac_sel_b = self.chip0.ab[1].c2;
                }
            }
            0xff20..=0xff3f => {
                let i = (addr - 0xff20) as usize % 4;
                self.chip1.ab[(i >> 1) & 1].write(i, data);

                match i {
                    0 if self.chip1.sound_enabled && !self.chip1.dac_sel_a && !self.chip1.dac_sel_b => {
                        let fdata = ((self.chip1.ab[0].read_output() >> 2) as f32 - 31.0) / 32.0;
                        self.push_audio_sample(fdata);
                    }
                    2 => {
                        let bit = self.chip1.ab[1].read_output() & 2 == 2;
                        if bit != self.chip1.last_bit_sound {
                            let fdata = if bit { 0.5 } else { -0.5 };
                            self.push_audio_sample(fdata);
                        }
                        self.chip1.last_bit_sound = bit;
                    }
                    3 => self.chip1.sound_enabled = data & 8 == 8,
                    _ => (),
                }
            }
            _ => (),
        }
    }

    fn push_audio_sample(&mut self, sample: f32) {
        if self.audio_buffer.len() < 1024 {
            self.audio_buffer.push_back(sample);
        }
    }

    /// Retrieve and clear collected audio samples
    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        self.audio_buffer.drain(..).collect()
    }

    // --- External Hardware Signals ---

    pub fn set_key(&mut self, key: u8, pressed: bool) {
        if let Some((row, col)) = self.ascii_to_matrix(key) {
            if pressed {
                self.chip0.col[col] |= 1 << row;
            } else {
                self.chip0.col[col] &= !(1 << row);
            }
            self.chip0.strobe_keyboard();
        }
    }

    pub fn hsync_irq(&mut self) -> bool {
        self.chip0.ab[0].set_c1(true);
        self.chip0.ab[0].consume_interrupt()
    }

    pub fn vsync_irq(&mut self) -> bool {
        self.chip0.ab[1].set_c1(true);
        self.chip0.ab[1].consume_interrupt()
    }

    pub fn cart_firq(&mut self) -> bool {
        self.chip1.ab[1].set_c1(true);
        self.chip1.ab[1].consume_interrupt()
    }

    pub fn get_vdg_bits(&self) -> u8 {
        (self.chip1.ab[1].read_data() >> 3) & 0x1f
    }

    fn ascii_to_matrix(&self, c: u8) -> Option<(usize, usize)> {
        match c.to_ascii_uppercase() {
            b'@' => Some((0, 0)), b'A' => Some((0, 1)), b'B' => Some((0, 2)), b'C' => Some((0, 3)),
            b'D' => Some((0, 4)), b'E' => Some((0, 5)), b'F' => Some((0, 6)), b'G' => Some((0, 7)),
            b'H' => Some((1, 0)), b'I' => Some((1, 1)), b'J' => Some((1, 2)), b'K' => Some((1, 3)),
            b'L' => Some((1, 4)), b'M' => Some((1, 5)), b'N' => Some((1, 6)), b'O' => Some((1, 7)),
            b'P' => Some((2, 0)), b'Q' => Some((2, 1)), b'R' => Some((2, 2)), b'S' => Some((2, 3)),
            b'T' => Some((2, 4)), b'U' => Some((2, 5)), b'V' => Some((2, 6)), b'W' => Some((2, 7)),
            b'X' => Some((3, 0)), b'Y' => Some((3, 1)), b'Z' => Some((3, 2)), b' ' => Some((3, 7)),
            b'0' => Some((4, 0)), b'1' => Some((4, 1)), b'2' => Some((4, 2)), b'3' => Some((4, 3)),
            b'4' => Some((4, 4)), b'5' => Some((4, 5)), b'6' => Some((4, 6)), b'7' => Some((4, 7)),
            b'8' => Some((5, 0)), b'9' => Some((5, 1)), b':' => Some((5, 2)), b';' => Some((5, 3)),
            b',' => Some((5, 4)), b'-' => Some((5, 5)), b'.' => Some((5, 6)), b'/' => Some((5, 7)),
            b'\r' | b'\n' => Some((6, 0)), 0x08 => Some((3, 5)),
            _ => None,
        }
    }
}
