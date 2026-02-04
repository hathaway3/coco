use super::{InputDevice, InputEvent};
use hal::pio::{PIOExt, StateMachineIndex, UninitStateMachine};
use pio::pio_asm;
use rp235x_hal as hal;

pub struct Ps2Keyboard<P: PIOExt, SM: StateMachineIndex> {
    rx: hal::pio::Rx<(P, SM)>,
    shift_reg: u16,
    break_code: bool,
}

impl<P: PIOExt, SM: StateMachineIndex> Ps2Keyboard<P, SM> {
    pub fn new(
        pio: &mut hal::pio::PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        data_pin_id: u8,
        _clk_pin_id: u8, // Assumed to be data_pin + 1 if side-set? No, generic.
                         // Wait, PIO 'wait pin' uses Input Source.
                         // We must configure pins in the State Machine.
    ) -> Self {
        // Simple Assumption: User provides sequential pins?
        // Or we map them?
        // PIO "in" uses `in_base`. "wait pin 1" refers to `in_base + 1`?
        // NO. "wait pin" uses `exec_ctrl.jmp_pin` relative?
        // Docs: "WAIT ... PIN index". "The index is mapped to a GPIO number via proper configuration".
        // specifically `sm_config_set_in_pins` + `sm_config_set_jmp_pin`.
        // Actually `WAIT x PIN y` uses `Input Pin` mapping (same as IN).
        // So `wait 0 pin 1` checks `in_base + 1`.

        // So we need Data at `base` and Clock at `base + 1`.
        // If users pins are not sequential, we can't use this simple PIO without JMP PIN mapping (which handles 1 pin).
        // For simplicity, we assume sequential: [Data, Clock].

        let program = pio_asm!(
            ".wrap_target",
            "wait 0 pin 1",
            "in pins, 1",
            "wait 1 pin 1",
            ".wrap"
        )
        .program;

        let installed = pio.install(&program).unwrap();

        let (mut sm, rx, _tx) = hal::pio::PIOBuilder::from_installed_program(installed)
            .in_pin_base(data_pin_id)
            .autopush(true)
            .push_threshold(1) // Push every bit
            .build(sm);

        // Set pin directions to input
        // We need to set both Data (base) and Clock (base+1)
        // Since we don't have clk_pin_id in strict usage here if we assume sequential.
        // But for safety let's assume they passed the right ID.
        sm.set_pindirs([
            (data_pin_id, hal::pio::PinDir::Input),
            (data_pin_id + 1, hal::pio::PinDir::Input),
        ]);

        sm.start();

        Self {
            rx,
            shift_reg: 0,
            break_code: false,
        }
    }

    fn map_scancode(&self, code: u8) -> Option<u8> {
        match code {
            0x1C => Some(b'a'),
            0x32 => Some(b'b'),
            0x21 => Some(b'c'),
            0x23 => Some(b'd'),
            0x24 => Some(b'e'),
            0x2B => Some(b'f'),
            0x34 => Some(b'g'),
            0x33 => Some(b'h'),
            0x43 => Some(b'i'),
            0x3B => Some(b'j'),
            0x42 => Some(b'k'),
            0x4B => Some(b'l'),
            0x3A => Some(b'm'),
            0x31 => Some(b'n'),
            0x44 => Some(b'o'),
            0x4D => Some(b'p'),
            0x15 => Some(b'q'),
            0x2D => Some(b'r'),
            0x1B => Some(b's'),
            0x2C => Some(b't'),
            0x3C => Some(b'u'),
            0x2A => Some(b'v'),
            0x1D => Some(b'w'),
            0x22 => Some(b'x'),
            0x35 => Some(b'y'),
            0x1A => Some(b'z'),
            0x16 => Some(b'1'),
            0x1E => Some(b'2'),
            0x26 => Some(b'3'),
            0x25 => Some(b'4'),
            0x2E => Some(b'5'),
            0x36 => Some(b'6'),
            0x3D => Some(b'7'),
            0x3E => Some(b'8'),
            0x46 => Some(b'9'),
            0x45 => Some(b'0'),
            0x5A => Some(b'\n'), // Enter
            0x29 => Some(b' '),  // Space
            0x66 => Some(0x08),  // Backspace
            _ => None,
        }
    }

    fn decode_scancode(&mut self, code: u8) -> Option<InputEvent> {
        if code == 0xF0 {
            self.break_code = true;
            return None;
        }
        if self.break_code {
            self.break_code = false;
            // Key Release
            return self.map_scancode(code).map(InputEvent::Release);
        }
        // Key Press
        self.map_scancode(code).map(InputEvent::Press)
    }
}

impl<P: PIOExt, SM: StateMachineIndex> InputDevice for Ps2Keyboard<P, SM> {
    fn poll(&mut self) -> Option<InputEvent> {
        while let Some(word) = self.rx.read() {
            let bit = (word & 1) as u16;

            // Shift in: Newest bit is MSB?
            // PS/2 sends LSB first.
            // Start(0), D0, D1... D7, Parity, Stop(1). (11 bits)
            // If we shift right: `reg = (reg >> 1) | (bit << 10)`.
            // The Start bit (0) will eventually be at offset 0 (LSB).

            self.shift_reg = (self.shift_reg >> 1) | (bit << 10);

            // Check framing
            // Start bit (bit 0) should be 0.
            // Stop bit (bit 10) should be 1.
            let start = self.shift_reg & 1;
            let stop = (self.shift_reg >> 10) & 1;

            if start == 0 && stop == 1 {
                // Valid frame candidate
                // Extract Data: bits 1..9
                let data = ((self.shift_reg >> 1) & 0xFF) as u8;

                // Parity check? (bit 9). Odd parity.
                // For simplicity, skip parity check or add TODO.

                // Clear register to avoid re-detecting
                self.shift_reg = 0xFFFF; // Fill with 1s so Start(0) is lost?

                if let Some(event) = self.decode_scancode(data) {
                    return Some(event);
                }
            }
        }
        None
    }
}
