#[cfg(target_os = "none")]
pub mod ps2;
pub mod usb;

/// Input Event
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Press(u8),
    Release(u8),
}

/// Trait for input devices (Keyboards) to implement.
pub trait InputDevice {
    /// Poll the device for new input.
    fn poll(&mut self) -> Option<InputEvent>;
}
