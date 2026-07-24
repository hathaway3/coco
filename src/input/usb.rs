use super::{InputDevice, InputEvent};

#[cfg(target_os = "none")]
use rp235x_hal::usb::UsbBus;
#[cfg(target_os = "none")]
use usb_device::bus::UsbBusAllocator;
#[cfg(target_os = "none")]
use usb_device::prelude::*;
#[cfg(target_os = "none")]
use usbd_serial::SerialPort;

pub struct UsbKeyboard {
    #[cfg(target_os = "none")]
    usb_dev: UsbDevice<'static, UsbBus>,
    #[cfg(target_os = "none")]
    serial: SerialPort<'static, UsbBus>,
    pressed_key: Option<u8>,
}

#[cfg(target_os = "none")]
impl UsbKeyboard {
    pub fn new(usb_bus: &'static UsbBusAllocator<UsbBus>) -> Self {
        let serial = SerialPort::new(usb_bus);

        let descriptors = [
            usb_device::device::StringDescriptors::new(usb_device::LangID::EN)
                .manufacturer("Antigravity")
                .product("CoCo Keyboard Bridge")
                .serial_number("COCO1"),
        ];

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .strings(&descriptors)
            .expect("Failed to set string descriptors")
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();
        Self {
            usb_dev,
            serial,
            pressed_key: None,
        }
    }

    pub fn read_serial(&mut self, buf: &mut [u8]) -> Result<usize, usb_device::UsbError> {
        self.serial.read(buf)
    }

    pub fn write_serial(&mut self, buf: &[u8]) -> Result<usize, usb_device::UsbError> {
        self.serial.write(buf)
    }

    pub fn poll_usb(&mut self) -> bool {
        self.usb_dev.poll(&mut [&mut self.serial])
    }
}

#[cfg(not(target_os = "none"))]
impl Default for UsbKeyboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "none"))]
impl UsbKeyboard {
    pub fn new() -> Self {
        Self { pressed_key: None }
    }
}

impl InputDevice for UsbKeyboard {
    fn poll(&mut self) -> Option<InputEvent> {
        // If a key was pressed in the previous poll, release it now.
        if let Some(k) = self.pressed_key.take() {
            return Some(InputEvent::Release(k));
        }

        // On embedded target, poll the hardware USB interface.
        #[cfg(target_os = "none")]
        {
            if self.usb_dev.poll(&mut [&mut self.serial]) {
                let mut buf = [0u8; 64];
                match self.serial.read(&mut buf) {
                    Ok(count) if count > 0 => {
                        let char_val = buf[0];
                        // Only register valid ASCII characters mapable to CoCo keyboard matrix
                        if char_val != 0 {
                            self.pressed_key = Some(char_val);
                            return Some(InputEvent::Press(char_val));
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }
}
