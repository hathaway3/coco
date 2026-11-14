use crate::sam::Sam;
use crate::vdg::{Vdg, VdgMode};
use crate::DISPLAY_BUFFER;
use alloc::sync::Arc;
use spin::Mutex;

// DeviceManager should be instantiated on the main thread and then clones of its
// member fields can be sent to other threads. DeviceManger methods must only be
// called on the main thread.
pub struct DeviceManager {
    pub display: &'static mut [u16],
    pub sam: Arc<Mutex<Sam>>,
    pub vdg: Arc<Mutex<Vdg>>,
    pub pia: Arc<Mutex<crate::pia::PiaPair>>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::with_vram_offset(0)
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_vram_offset(vram_offset: usize) -> Self {
        let vdg = Arc::new(Mutex::new(Vdg::with_vram_offset(vram_offset)));
        let pia = Arc::new(Mutex::new(crate::pia::PiaPair::new()));
        DeviceManager {
            display: unsafe { &mut (*core::ptr::addr_of_mut!(DISPLAY_BUFFER)).0 },
            sam: Arc::new(Mutex::new(Sam::new())),
            vdg,
            pia,
        }
    }

    pub fn get_vdg(&self) -> Arc<Mutex<Vdg>> {
        self.vdg.clone()
    }
    pub fn get_pia(&self) -> Arc<Mutex<crate::pia::PiaPair>> {
        self.pia.clone()
    }
    pub fn get_sam(&self) -> Arc<Mutex<Sam>> {
        self.sam.clone()
    }
    pub fn is_running(&self) -> bool {
        true
    }

    pub fn update(&mut self) {
        let mode;
        let css;
        let vram_offset;
        {
            // use SAM and PIA pair to determine current VDG mode
            let pia = self.pia.lock();
            let sam = self.sam.lock();
            let pia_bits = pia.get_vdg_bits();
            mode = VdgMode::try_from_pia_and_sam(pia_bits, sam.get_vdg_bits());
            css = pia_bits & 1 == 1;
            // get the starting address of VRAM from the SAM
            vram_offset = sam.get_vram_start() as usize;
        }
        // only try rendering the screen if we have a valid VdgMode
        if let Some(mode) = mode {
            let mut vdg = self.vdg.lock();
            vdg.set_mode(mode);
            vdg.set_vram_offset(vram_offset);
            // convert contents of VRAM to pixels for display
            vdg.render(self.display, css);
        }
    }
}
