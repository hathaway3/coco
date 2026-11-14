#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
mod embedded {
    // Import everything from the crate root
    use coco::*;

    // Embedded-specific imports
    // Import dependencies
    use core::arch::global_asm;
    use core::mem::MaybeUninit;
    use core::ptr::addr_of_mut;
    use defmt_rtt as _;
    use embedded_alloc::Heap;
    use embedded_hal::digital::InputPin;
    use hal::clocks::ClockSource;
    use hal::fugit::RateExtU32;
    use hal::pac;
    use panic_probe as _;
    use rp235x_hal as hal;

    global_asm! {
        include_str!("../vendor/pico-dvi-rs/src/pre_init.asm"),
        options(raw)
    }

    /// Tell the RP2350 Boot ROM about our application (must reside in .start_block within first 4KB of flash)
    #[link_section = ".start_block"]
    #[used]
    pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

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

    // SD Card Support
    use embedded_sdmmc::{Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};

    // Custom SpiDevice wrapper for embedded-hal 1.0 SPI
    struct SimpleSpiDevice<SPI, CS> {
        spi: SPI,
        cs: CS,
    }

    impl<SPI, CS> embedded_hal::spi::ErrorType for SimpleSpiDevice<SPI, CS>
    where
        SPI: embedded_hal::spi::ErrorType,
    {
        type Error = SPI::Error;
    }

    impl<SPI, CS> embedded_hal::spi::SpiDevice for SimpleSpiDevice<SPI, CS>
    where
        SPI: embedded_hal::spi::SpiBus,
        CS: embedded_hal::digital::OutputPin,
    {
        fn transaction(
            &mut self,
            operations: &mut [embedded_hal::spi::Operation<'_, u8>],
        ) -> Result<(), Self::Error> {
            let _ = self.cs.set_low();
            for op in operations {
                match op {
                    embedded_hal::spi::Operation::Read(buffer) => {
                        self.spi.read(buffer)?;
                    }
                    embedded_hal::spi::Operation::Write(buffer) => {
                        self.spi.write(buffer)?;
                    }
                    embedded_hal::spi::Operation::Transfer(read, write) => {
                        self.spi.transfer(read, write)?;
                    }
                    embedded_hal::spi::Operation::TransferInPlace(buffer) => {
                        self.spi.transfer_in_place(buffer)?;
                    }
                    embedded_hal::spi::Operation::DelayNs(ns) => {
                        cortex_m::asm::delay(*ns / 4);
                    }
                }
            }
            let _ = self.cs.set_high();
            Ok(())
        }
    }

    // Delay wrapper for embedded-sdmmc
    struct TimerDelay<'a> {
        timer: &'a hal::Timer<hal::timer::CopyableTimer0>,
    }

    impl<'a> embedded_hal::delay::DelayNs for TimerDelay<'a> {
        fn delay_ns(&mut self, ns: u32) {
            let us = ns.div_ceil(1000);
            let start = self.timer.get_counter().ticks();
            while (self.timer.get_counter().ticks() - start) < us as u64 {
                core::hint::spin_loop();
            }
        }
    }

    // Dummy TimeSource for FAT file system
    struct DummyTimeSource;
    impl TimeSource for DummyTimeSource {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 56,   // 2026
                zero_indexed_month: 6, // July (0-indexed is 6)
                zero_indexed_day: 19,  // 20th (0-indexed is 19)
                hours: 1,
                minutes: 0,
                seconds: 0,
            }
        }
    }

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
            2_u32.div_ceil(HSTX_MULTIPLE),
        );

        let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &_clocks);

        // Initialize PWM for audio on GPIO 0
        let pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
        let mut pwm = pwm_slices.pwm0;
        pwm.set_top(255);
        pwm.enable();

        let sio = hal::Sio::new(pac.SIO);
        let pins = hal::gpio::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        let mut channel_a = pwm.channel_a;
        channel_a.output_to(pins.gpio0);

        fn update_audio_pwm(val: u8) {
            unsafe {
                (*hal::pac::PWM::ptr())
                    .ch(0)
                    .cc()
                    .write(|w| w.a().bits(val as u16));
            }
        }

        coco::set_audio_callback(update_audio_pwm);

        // Initialize ADC for Joystick analog axes
        let mut adc = hal::adc::Adc::new(pac.ADC, &mut pac.RESETS);
        let mut joy_x_pin = hal::adc::AdcPin::new(pins.gpio26.into_floating_input()).unwrap();
        let mut joy_y_pin = hal::adc::AdcPin::new(pins.gpio27.into_floating_input()).unwrap();

        // Initialize Joystick Button GPIOs with Pull-Up
        let mut joy_btn1_pin = pins.gpio2.into_pull_up_input();
        let mut joy_btn2_pin = pins.gpio3.into_pull_up_input();

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
        let ps2_data = pins
            .gpio28
            .into_pull_up_input()
            .into_function::<hal::gpio::FunctionPio0>();
        let ps2_clk = pins
            .gpio29
            .into_pull_up_input()
            .into_function::<hal::gpio::FunctionPio0>();

        // Initialize PS/2 Keyboard Driver
        let mut ps2_kb = Ps2Keyboard::new(&mut pio0, sm0, ps2_data.id().num, ps2_clk.id().num);

        // Initialize USB Keyboard Driver (CDC Serial ACM)
        static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<rp235x_hal::usb::UsbBus>> =
            None;
        let usb_bus = usb_device::bus::UsbBusAllocator::new(rp235x_hal::usb::UsbBus::new(
            pac.USB,
            pac.USB_DPRAM,
            _clocks.usb_clock,
            true,
            &mut pac.RESETS,
        ));
        unsafe {
            USB_BUS = Some(usb_bus);
        }
        let usb_bus_ref = unsafe {
            #[allow(static_mut_refs)]
            USB_BUS.as_ref().unwrap()
        };
        let mut usb_kb = UsbKeyboard::new(usb_bus_ref);

        // --- Emulator Core Initialization ---
        let mut dm = DeviceManager::new();
        let mut core = Core::new(
            unsafe { &mut *coco::RAM_DISK.as_mut_ptr() },
            dm.sam.clone(),
            dm.vdg.clone(),
            dm.pia.clone(),
            0x8000,
            Some(0xFF40),
        );

        // --- SD Card & ROM Loading ---
        let mut loaded_color = false;
        let mut loaded_extended = false;
        let mut loaded_cart = false;

        {
            // Configure SPI0:
            // SCK: GPIO 6
            // MOSI: GPIO 7
            // MISO: GPIO 4
            // CS: GPIO 5
            let spi_sck = pins.gpio6.into_function::<hal::gpio::FunctionSpi>();
            let spi_mosi = pins.gpio7.into_function::<hal::gpio::FunctionSpi>();
            let spi_miso = pins.gpio4.into_function::<hal::gpio::FunctionSpi>();
            let spi_cs = pins
                .gpio5
                .into_push_pull_output_in_state(hal::gpio::PinState::High);

            let spi = hal::Spi::<_, _, _, 8>::new(pac.SPI0, (spi_mosi, spi_miso, spi_sck));
            let spi = spi.init(
                &mut pac.RESETS,
                _clocks.peripheral_clock.get_freq(),
                4_000_000_u32.Hz(),
                embedded_hal::spi::MODE_0,
            );

            let spi_dev = SimpleSpiDevice { spi, cs: spi_cs };
            let delay_prov = TimerDelay { timer: &timer };
            let sdcard = SdCard::new(spi_dev, delay_prov);
            let mut volume_mgr = VolumeManager::new(sdcard, DummyTimeSource);

            defmt::info!("Initializing SD Card...");
            match volume_mgr.open_volume(VolumeIdx(0)) {
                Ok(mut volume) => {
                    defmt::info!("SD Card Volume 0 mounted successfully.");
                    match volume.open_root_dir() {
                        Ok(mut root_dir) => {
                            defmt::info!("SD Card Root Directory opened.");

                            // Helper function to read a file from the root directory into CoCo RAM/ROM
                            let mut read_rom_file = |filename: &str, target_addr: u16| -> bool {
                                match root_dir.open_file_in_dir(filename, Mode::ReadOnly) {
                                    Ok(mut file) => {
                                        defmt::info!("Loading {} from SD Card...", filename);
                                        let mut offset = 0;
                                        loop {
                                            let mut read_buf = [0u8; 1024];
                                            match file.read(&mut read_buf) {
                                                Ok(0) => break,
                                                Ok(n) => {
                                                    if core
                                                        .load_bytes(
                                                            &read_buf[0..n],
                                                            target_addr + offset,
                                                        )
                                                        .is_err()
                                                    {
                                                        defmt::error!("Failed to write bytes to emulated memory at {:x}", target_addr + offset);
                                                        return false;
                                                    }
                                                    offset += n as u16;
                                                }
                                                Err(e) => {
                                                    defmt::error!(
                                                        "Error reading file {}: {:?}",
                                                        filename,
                                                        defmt::Debug2Format(&e)
                                                    );
                                                    return false;
                                                }
                                            }
                                        }
                                        defmt::info!(
                                            "Successfully loaded {} ({} bytes)",
                                            filename,
                                            offset
                                        );
                                        true
                                    }
                                    Err(_) => {
                                        defmt::info!("File {} not found on SD card.", filename);
                                        false
                                    }
                                }
                            };

                            loaded_color = read_rom_file("COLOR.ROM", 0xA000);
                            loaded_extended = read_rom_file("EXTENDED.ROM", 0x8000);
                            loaded_cart = read_rom_file("CART.ROM", 0xC000);
                        }
                        Err(e) => {
                            defmt::error!(
                                "Failed to open SD Card Root Directory: {:?}",
                                defmt::Debug2Format(&e)
                            );
                        }
                    }
                }
                Err(e) => {
                    defmt::info!(
                        "SD Card not found or failed to mount: {:?}",
                        defmt::Debug2Format(&e)
                    );
                }
            };
        }

        if loaded_color {
            defmt::info!("Booting CoCo with loaded Color BASIC ROM...");
            if loaded_extended {
                defmt::info!("Extended Color BASIC ROM loaded.");
            }
            if loaded_cart {
                defmt::info!("Cartridge ROM loaded.");
            }
            core.reset().unwrap();
        } else {
            defmt::info!("No boot ROM loaded. Booting with placeholder test ROM...");
            let dummy_rom = [0x12, 0x12, 0x20, 0xFE];
            core.load_bytes(&dummy_rom, 0xA000).unwrap();
            core.force_reset_vector(0xA000).unwrap();
            core.reset().unwrap();
        }

        // --- DVI Display List Initialization ---
        let mut current_border_color = 0x0000u16;

        fn build_dvi_display_list(dm: &mut DeviceManager, border_color: u16) {
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

            let border_u32 = ((border_color as u32) << 16) | (border_color as u32);

            // Top Margin
            rb.begin_stripe(v_margin);
            rb.end_stripe();
            sb.begin_stripe(v_margin);
            sb.solid(640, border_u32);
            sb.end_stripe();

            // Active Area: 192 VDG lines, each output twice (384 scanlines)
            // We use height=2 stripes with stride=0 so the render list automatically reads the same VDG line twice.
            for line in 0..v_active {
                let start_idx = (line as usize) * words_per_line;
                let line_slice = &display_u32[start_idx..start_idx + words_per_line];

                // Render: copy raw RGB555 pixel data into LineBuf using stripe height 2 and stride 0
                rb.begin_stripe(2);
                rb.blit_block(line_slice, words_per_line as u32, 0);
                rb.end_stripe();

                // Scan: margin, pixel data, margin
                sb.begin_stripe(2);
                sb.solid(h_margin, border_u32);
                sb.copy_pixels(h_active as u32);
                sb.solid(h_margin, border_u32);
                sb.end_stripe();
            }

            // Bottom Margin
            rb.begin_stripe(v_margin);
            rb.end_stripe();
            sb.begin_stripe(v_margin);
            sb.solid(640, border_u32);
            sb.end_stripe();

            end_display_list(rb, sb);
        }

        build_dvi_display_list(&mut dm, current_border_color);

        let mut base_us = timer.get_counter().ticks();
        let mut base_cycles = core.clock_cycles;

        // Main Emulator Loop
        loop {
            // Run core for a slice of cycles
            for _ in 0..200 {
                if let Err(_e) = core.exec_one() {
                    break;
                }
            }

            // Throttling to match target CPU speed (~0.895 MHz)
            if core.clock_cycles < base_cycles {
                base_cycles = core.clock_cycles;
                base_us = timer.get_counter().ticks();
            }
            let elapsed_cycles = core.clock_cycles - base_cycles;
            let expected_elapsed_us = (elapsed_cycles * 1000000) / 894886;
            while (timer.get_counter().ticks() - base_us) < expected_elapsed_us {
                core::hint::spin_loop();
            }
            // Update devices

            // Poll Input Devices
            if let Some(event) = ps2_kb.poll() {
                let mut pia = dm.pia.lock();
                match event {
                    InputEvent::Press(k) => pia.set_key(k, true),
                    InputEvent::Release(k) => pia.set_key(k, false),
                }
            }
            // DriveWire / ACIA & USB CDC Serial Routing
            let mut acia_accessed = false;
            if let Some(acia_rc) = &core.acia {
                if acia_rc.borrow().accessed {
                    acia_accessed = true;
                }
            }

            if acia_accessed {
                if let Some(acia_rc) = &core.acia {
                    let mut acia = acia_rc.borrow_mut();

                    // 1. Poll the USB stack
                    usb_kb.poll_usb();

                    // 2. Read any incoming bytes from USB CDC ACM serial and feed to ACIA
                    if acia.has_rx_space() {
                        let mut rx_buf = [0u8; 1];
                        if let Ok(1) = usb_kb.read_serial(&mut rx_buf) {
                            acia.rx_byte(rx_buf[0]);
                        }
                    }

                    // 3. Write any outgoing bytes from ACIA to USB CDC ACM serial
                    if let Some(tx_byte) = acia.take_tx_byte() {
                        let _ = usb_kb.write_serial(&[tx_byte]);
                    }
                }
            } else {
                if let Some(event) = usb_kb.poll() {
                    let mut pia = dm.pia.lock();
                    match event {
                        InputEvent::Press(k) => pia.set_key(k, true),
                        InputEvent::Release(k) => pia.set_key(k, false),
                    }
                }
            }

            // Poll Joystick Input (read ADC and Buttons)
            let (joy_x, joy_y) = {
                let x_raw = adc.read(&mut joy_x_pin).unwrap_or(2048);
                let y_raw = adc.read(&mut joy_y_pin).unwrap_or(2048);
                ((x_raw / 16) as u8, (y_raw / 16) as u8)
            };

            let joy_btn1 = joy_btn1_pin.is_low().unwrap_or(false);
            let joy_btn2 = joy_btn2_pin.is_low().unwrap_or(false);

            {
                let mut pia = dm.pia.lock();
                pia.set_joystick(joy_x, joy_y, joy_btn1, joy_btn2);
            }

            if core.vram_dirty {
                dm.vdg.lock().set_dirty();
                core.vram_dirty = false;
            }
            dm.update();

            let border_color = {
                let css = {
                    let pia = dm.pia.lock();
                    (pia.get_vdg_bits() & 1) == 1
                };
                let vdg = dm.vdg.lock();
                vdg.get_border_color(css).to_rgb555()
            };

            if border_color != current_border_color {
                current_border_color = border_color;
                build_dvi_display_list(&mut dm, border_color);
            }
        }
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("This firmware is intended for the RP2350 microcontroller. Use `cargo build --target thumbv8m.main-none-eabihf` to build.");
}
