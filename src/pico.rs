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
    const HEAP_SIZE: usize = 128 * 1024; // 128KB heap
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

    // Core 1 Stack
    use hal::multicore::{Multicore, Stack};
    static mut CORE1_STACK: Stack<1024> = Stack::new();
    const HSTX_MULTIPLE: u32 = 2; // From pico-dvi-rs

    // Import renderer functions
    use pico_dvi_rs::render::{end_display_list, init_display_swapcell, start_display_list};

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
            // Pinout for Adafruit Feather RP2350 (Adjust if using different board!)
            let pinout = DviPinout::new([D2, Clk, D1, D0], DviPolarity::Pos);

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
            dm.update();

            // Build DVI Display List
            let (mut rb, mut sb) = start_display_list();

            // Standard VGA 640x480. We use 320x240 logic (doubled by DVI driver repeat)
            // But we need to output 640 pixels per line horizontally if HSTX is 1:1?
            // Wait, HSTX multiple is 2. `bit_clk / HSTX_MULTIPLE`.
            // So pixel clock is efficient.
            // If I output 320 pixels, and DVI engine expects 640?
            // Actually `init_display_swapcell(width)`. Width is likely 640.
            // `video_scan_copy_16` copies pixels.
            // If I want 256 pixels centered:
            // 640 width.
            // VDG: 256 pixels.
            // Margins: (640-256)/2 = 192.

            // Render logic:
            // Lines 0..24: Black (solid)
            // Lines 24..216: Black(192), VDG(256), Black(192)
            // Lines 216..240: Black (solid)
            // (Total 240 lines, vertical repeat = 2 => 480)

            // Margin scanlines
            let v_margin = 24;
            // Top Margin
            rb.begin_stripe(v_margin);
            rb.end_stripe();
            sb.begin_stripe(v_margin);
            sb.solid(640, 0); // Black
            sb.end_stripe();

            // Active Area (192 lines)
            let v_active = 192;
            let h_margin = 192;
            let h_active = 256;

            rb.begin_stripe(v_active);

            // VDG Framebuffer view as u32
            let display_u32 = unsafe {
                core::slice::from_raw_parts(dm.display.as_ptr() as *const u32, dm.display.len() / 2)
            };

            // Blit lines
            // Since rb.tile64 operates relatively, we need to loop?
            // No, we can just say "for each line in stripe, do X"?
            // Wait. `Renderlist` contains instructions.
            // `tile64` adds instructions to blit ONE TILE row?
            // `dvi_main` loop: `scan_render.render_scanline` executes the list for ONE LINE.
            // `Renderlist` is re-executed for every scanline in the stripe!
            // Crucial: The Renderlist is STATIC for the stripe height.
            // If I use `tile64`, it blits the SAME data for every line in the stripe.
            // This is bad for a full bitmap!
            // `pico-dvi-rs` design assumes TILES (sprites) or TEXT where the same pattern repeats?
            // OR, `ScanRender` advances the source pointer?
            // `render_scanline`:
            // `render_engine` takes `render_ptr`.
            // `render_engine` executes.
            // `self.render_y += 1`.
            // `if self.render_y == stripe_height { ... }`
            // It seems `Renderlist` is designed for sprites.
            // BUT: `init_display_swapcell`?
            // If I want a full bitmap, I might need 192 stripes of height 1.
            // That would accept unique data for each line.

            // Let's do 192 stripes of height 1.
            // It is less efficient than 1 stripe, but required for unique content per line if renderlist doesn't auto-advance source.
            // `tile64` takes `tile`. `tile` is a slice.
            // If I create a Renderlist with 192 "stripes", each blitting a different slice of VDG.
            // That works.

            // End the top margin stripe first (done above).

            // Active Area Loop
            for line in 0..v_active {
                rb.begin_stripe(1);
                // Left margin (implicitly 0/black if we don't draw?)
                // `render_engine` zeroes the buffer?
                // `LineBuf::zero()` is static.
                // `render_scanline` calls `render_engine`.
                // `render_engine` usually clears or overwrites?
                // If we don't write, it likely retains garbage or prev line?
                // `ScanRender` does NOT clear `LINE_BUF`.
                // Converting `tile64`... we should probably clear or ensure full overwrite.
                // `video_scan_copy_16` copies whatever is in the buffer.
                // If I only blit central 256 pixels, sides might be garbage.
                // I should fill sides with black?
                // `rb` doesn't have `solid`.
                // But I can blit a "black" tile.
                // Let's assuming starting with black is hard.
                // Maybe I can just use `sb.solid` for margins?
                // `Scanlist` runs AFTER `render_engine`.
                // `Scanlist` instructions: `solid(192)`, `copy(256)`, `solid(192)`.
                // Yes! I can mix.

                // Render: Blit 256 pixels (128 u32s) to index 0?
                // Wait. `copy_pixels` copies from START of line buffer.
                // So I must render the VDG pixels at the START of `LineBuf` (x=0).
                // Then `Scanlist` will: `solid(192)`, `copy(256)`, `solid(192)`.
                // Wait. `copy` copies TMDS? No, `copy` copies FROM `LineBuf`.
                // If I `solid(192)` first, that emits 192 pixels of TMDS. `LineBuf` pointer is NOT advanced (unless I added code for that? No, `solid` uses r5/count).
                // Then `copy(256)`. This reads 256 pixels from `LineBuf`.
                // Since `line_buf_ptr` passed to `video_scan` is the start.
                // `video_scan_copy_16` reads from `r1` (input).
                // `r1` is updated by `ldmia r1!`.
                // So if I call `copy` multiple times, it advances.
                // But `solid` does NOT touch `r1`.
                // So: `solid(192)` (uses no input). `copy(256)` (consumes 256 input pixels). `solid(192)` (uses no input).
                // This implies `LineBuf` only needs to contain the 256 pixels of content!
                // So I render to `x=0` in `Renderlist`.

                // Slice for this line:
                let start_idx = (line as usize) * (h_active / 2); // u32 index
                let end_idx = start_idx + (h_active / 2);
                let line_slice = &display_u32[start_idx..end_idx];

                rb.tile64(line_slice, 0, 128); // Blit 128 u32s (256 pixels)
                rb.end_stripe();

                sb.begin_stripe(1);
                sb.solid(h_margin, 0); // Left 192
                sb.copy_pixels(h_active as u32); // Center 256
                sb.solid(h_margin, 0); // Right 192
                sb.end_stripe();
            }

            // Bottom Margin
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
