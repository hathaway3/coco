use super::*;
use crate::pia::*;
use crate::sam::*;
use spin::Mutex;
// use crate::sound;
use crate::vdg::*;

// DeviceManager should be instantiated on the main thread and then clones of its
// member fields can be sent to other threads. DeviceManger methods must only be
// called on the main thread.
pub struct DeviceManager {
    pub display: &'static mut [u16],
    pub ram: &'static mut [u8],
    pub sam: Arc<Mutex<Sam>>,
    pub vdg: Arc<Mutex<Vdg>>,
    pub pia0: Arc<Mutex<Pia0>>,
    pub pia1: Arc<Mutex<Pia1>>,
}

impl DeviceManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_ram(unsafe { &mut *(&raw mut RAM_DISK) }, 0)
    }

    pub fn with_ram(ram: &'static mut [u8], vram_offset: usize) -> Self {
        let vdg = Arc::new(Mutex::new(Vdg::with_ram(vram_offset)));
        let pia1 = Arc::new(Mutex::new(Pia1::new()));
        DeviceManager {
            display: unsafe { &mut *(&raw mut DISPLAY_BUFFER) },
            ram,
            sam: Arc::new(Mutex::new(Sam::new())),
            vdg,
            pia0: Arc::new(Mutex::new(Pia0::new(pia1.clone()))),
            pia1,
        }
    }

    pub fn get_vdg(&self) -> Arc<Mutex<Vdg>> {
        self.vdg.clone()
    }
    pub fn get_pia0(&self) -> Arc<Mutex<Pia0>> {
        self.pia0.clone()
    }
    pub fn get_pia1(&self) -> Arc<Mutex<Pia1>> {
        self.pia1.clone()
    }
    pub fn get_ram(&self) -> &'static mut [u8] {
        unsafe { &mut *(&raw mut RAM_DISK) }
    }
    pub fn get_sam(&self) -> Arc<Mutex<Sam>> {
        self.sam.clone()
    }
    pub fn is_running(&self) -> bool {
        true
    }

    pub fn update(&mut self) {
        let mut _redraw = false;
        {
            // pia0 handles keyboard input
            let mut _pia0 = self.pia0.lock();
            // pia0.update(&self.window);
        }
        let mode;
        let css;
        let vram_offset;
        {
            // use SAM and PIA1 to determine current VDG mode
            let sam = self.sam.lock();
            let pia1 = self.pia1.lock();
            let pia_bits = pia1.get_vdg_bits();
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
            _redraw = vdg.render(&mut self.display, css);
        }
        /*
        if redraw {
            self.window
                .update_with_buffer(&self.display, SCREEN_DIM_X, SCREEN_DIM_Y)
                .expect("minifb update_with_buffer failed");
        } else {
            self.window.update();
        }
        */
    }
}
