#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
mod embedded {
    // Import everything from the crate root
    use coco::*;

    // Embedded-specific imports
    // Import dependencies
    use core::mem::MaybeUninit;
    use core::ptr::addr_of_mut;
    use defmt_rtt as _;
    use embedded_alloc::Heap;
    use hal::pac;
    use panic_probe as _;
    use rp235x_hal as hal;

    // Import pico-dvi-rs library
    use pico_dvi_rs::dvi; // Module for setup functions
    use pico_dvi_rs::{
        clock::init_clocks,
        dvi::{
            core1_main,
            pinout::{DviPinout, DviPolarity},
            timing::VGA_TIMING,
            DviInst,
        },
        DVI_INST,
    };

    // Allocate a heap for embedded-alloc
    #[global_allocator]
    static HEAP: Heap = Heap::empty();
    const HEAP_SIZE: usize = 96 * 1024; // 96KB heap (reduced from 128KB for better memory efficiency)
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

    // Core 1 Stack
    use hal::multicore::{Multicore, Stack};
    static mut CORE1_STACK: Stack<1024> = Stack::new();
    const HSTX_MULTIPLE: u32 = 2; // From pico-dvi-rs

    // Import renderer functions
    use pico_dvi_rs::render::{end_display_list, init_display_swapcell, start_display_list};

    // Input Support
    use coco::input::{ps2::Ps2Keyboard, usb::UsbKeyboard, InputDevice, InputEvent};
    use hal::pio::PIOExt;

    #[rp235x_hal::entry]
    fn main() -> ! {
        // ... (setup skipped) ...
        // Initialize heap
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }

        let mut pac = pac::Peripherals::take().unwrap();
        let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

        // Initialise the clocks (Custom DVI Clock Setup)
        let timing = VGA_TIMING;
        let width = timing.h_active_pixels; // Capture width before move
        let _clocks = init_clocks(
            pac.XOSC,
            pac.ROSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
            timing.bit_clk / HSTX_MULTIPLE,
            2 / HSTX_MULTIPLE,
        );

        let sio = hal::Sio::new(pac.SIO);
        let pins = hal::gpio::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        defmt::info!("CoCo Emulator starting on RP2350...");

        // --- DVI Initialization ---
        let gpio_pin = pins
            .gpio10
            .into_push_pull_output_in_state(hal::gpio::PinState::Low);

        // Initialize DVI Instance (Global)
        unsafe {
            (*DVI_INST.0.get()).write(DviInst::new(timing, gpio_pin));

            // Peripheral safety checks (from library main.rs)
            let periphs = hal::pac::Peripherals::steal();
            periphs.RESETS.reset().modify(|_, w| w.hstx().clear_bit());
            while periphs.RESETS.reset_done().read().hstx().bit_is_clear() {}

            use pico_dvi_rs::dvi::pinout::DviPair::*;
            // Pinout for Raspberry Pi Pico 2 W (GPIO 12-19 in standard HSTX order)
            // GPIO 12-13: D0, GPIO 14-15: CLK, GPIO 16-17: D2, GPIO 18-19: D1
            let pinout = DviPinout::new([D0, Clk, D2, D1], DviPolarity::Pos);

            dvi::setup_hstx(&periphs.HSTX_CTRL, pinout);
            dvi::setup_dma(&periphs.DMA, &periphs.HSTX_FIFO);

            // Boost DMA priority
            periphs
                .BUSCTRL
                .bus_priority()
                .write(|w| w.dma_r().set_bit().dma_w().set_bit());

            dvi::setup_pins(&periphs.PADS_BANK0, &periphs.IO_BANK0);
        }

        // Initialize Display Swap Mechanism
        init_display_swapcell(width);

        // Start Core 1 for Video Signal Generation
        let mut fifo = sio.fifo;
        let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut fifo);
        let cores = mc.cores();
        let core1 = &mut cores[1];
        core1
            .spawn(
                unsafe {
                    #[allow(static_mut_refs)]
                    CORE1_STACK.take().unwrap()
                },
                move || core1_main(),
            )
            .unwrap();

        defmt::info!("DVI signal started on Core 1.");

        // --- Input Initialization ---
        // Split PIO0 for PS/2 (and potentially USB later)
        let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

        // Configure PS/2 Pins (GPIO 28 = Data, GPIO 29 = Clock)
        let ps2_data = pins.gpio28.into_function::<hal::gpio::FunctionPio0>();
        let ps2_clk = pins.gpio29.into_function::<hal::gpio::FunctionPio0>();

        // Initialize PS/2 Keyboard Driver
        let mut ps2_kb = Ps2Keyboard::new(&mut pio0, sm0, ps2_data.id().num, ps2_clk.id().num);

        // Initialize USB Keyboard Driver (Placeholder)
        let mut usb_kb = UsbKeyboard::new();

        // --- Emulator Core Initialization ---
        let mut dm = DeviceManager::new();
        let mut core = Core::new(
            unsafe { &mut *(&raw mut RAM_DISK) },
            dm.sam.clone(),
            dm.vdg.clone(),
            dm.pia0.clone(),
            dm.pia1.clone(),
            0x8000,
            None,
        );

        // Load a placeholder ROM
        let dummy_rom = [0x12, 0x12, 0x12, 0xFE];
        core.load_bytes(&dummy_rom, 0xA000).unwrap();
        core.force_reset_vector(0xA000).unwrap();
        core.reset().unwrap();

        // Main Emulator Loop
        loop {
            // Run core for a slice of cycles
            for _ in 0..10000 {
                // Increased slice
                if let Err(_e) = core.exec_one() {
                    break;
                }
            }
            // Update devices

            // Poll Input Devices
            if let Some(event) = ps2_kb.poll() {
                match event {
                    InputEvent::Press(k) => dm.pia0.lock().set_key(k, true),
                    InputEvent::Release(k) => dm.pia0.lock().set_key(k, false),
                }
            }
            if let Some(event) = usb_kb.poll() {
                match event {
                    InputEvent::Press(k) => dm.pia0.lock().set_key(k, true),
                    InputEvent::Release(k) => dm.pia0.lock().set_key(k, false),
                }
            }

            dm.update();

            // Build DVI Display List
            // VGA 640x480 @ 60Hz. VERTICAL_REPEAT=1, so we must produce all 480 scanlines.
            // VDG renders 256x192 pixels in RGB555 format.
            // We center 256 pixels horizontally (192px black margin each side)
            // and double each of 192 VDG lines vertically (2x) for 384 active lines.
            // Layout: 48 top margin + 384 active + 48 bottom margin = 480 lines.
            let (mut rb, mut sb) = start_display_list();

            let v_margin = 48u32; // Top/bottom margin (480 - 384) / 2
            let v_active = 192u32; // VDG lines (each output twice = 384 scanlines)
            let h_margin = 192u32; // Left/right margin (640 - 256) / 2
            let h_active = 256usize;
            let words_per_line = h_active / 2; // 128 u32s per VDG line (2 RGB555 pixels per u32)

            // VDG Framebuffer viewed as u32 (pairs of RGB555 pixels)
            let display_u32 = unsafe {
                core::slice::from_raw_parts(dm.display.as_ptr() as *const u32, dm.display.len() / 2)
            };

            // Top Margin (solid black)
            rb.begin_stripe(v_margin);
            rb.end_stripe();
            sb.begin_stripe(v_margin);
            sb.solid(640, 0);
            sb.end_stripe();

            // Active Area: 192 VDG lines, each output twice (384 scanlines)
            for line in 0..v_active {
                let start_idx = (line as usize) * words_per_line;
                let line_slice = &display_u32[start_idx..start_idx + words_per_line];

                // Output each VDG line twice for 2x vertical scaling
                for _repeat in 0..2 {
                    // Render: copy raw RGB555 pixel data into LineBuf
                    rb.begin_stripe(1);
                    rb.blit_1bpp(line_slice, words_per_line, 1);
                    rb.end_stripe();

                    // Scan: black margin, pixel data, black margin
                    sb.begin_stripe(1);
                    sb.solid(h_margin, 0);
                    sb.copy_pixels(h_active as u32);
                    sb.solid(h_margin, 0);
                    sb.end_stripe();
                }
            }

            // Bottom Margin (solid black)
            rb.begin_stripe(v_margin);
            rb.end_stripe();
            sb.begin_stripe(v_margin);
            sb.solid(640, 0);
            sb.end_stripe();

            end_display_list(rb, sb);
        }
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("This firmware is intended for the RP2350 microcontroller. Use `cargo build --target thumbv8m.main-none-eabihf` to build.");
}
