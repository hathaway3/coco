# RP2350 (Pico 2) Hardware Reference: Rusty# CoCo

This document details the hardware configuration, pin mappings, and multicore architecture for the RP2350 (Raspberry Pi Pico 2 W) target.

## Pin Mappings (GPIO)

| Pin | Function | Description |
| --- | --- | --- |
| **GPIO 10** | **Heartbeat** | General status LED / Heartbeat indicator. |
| **GPIO 12** | **DVI D0+** | HSTX Lane 0 positive. |
| **GPIO 13** | **DVI D0-** | HSTX Lane 0 negative. |
| **GPIO 14** | **DVI CLK+** | HSTX Clock positive. |
| **GPIO 15** | **DVI CLK-** | HSTX Clock negative. |
| **GPIO 16** | **DVI D2+** | HSTX Lane 2 positive. |
| **GPIO 17** | **DVI D2-** | HSTX Lane 2 negative. |
| **GPIO 18** | **DVI D1+** | HSTX Lane 1 positive. |
| **GPIO 19** | **DVI D1-** | HSTX Lane 1 negative. |
| **GPIO 0** | **Audio Out** | PWM Audio output (PWM 0, Channel A). |
| **GPIO 2** | **Joy Button 1**| Joystick Button 1 (Pull-up Input). |
| **GPIO 3** | **Joy Button 2**| Joystick Button 2 (Pull-up Input). |
| **GPIO 26**| **Joy X Axis**  | Analog Joystick X-Axis (ADC 0). |
| **GPIO 27**| **Joy Y Axis**  | Analog Joystick Y-Axis (ADC 1). |
| **GPIO 28**| **PS/2 Data**   | PS/2 Keyboard data line (PIO 0). |
| **GPIO 29**| **PS/2 Clock**  | PS/2 Keyboard clock line (PIO 0). |
| **USB**     | **USB Keyboard**| Native CDC ACM USB Serial Keyboard Bridge. |

## Multicore Architecture

The Rusty# CoCo emulator utilizes both ARM Cortex-M33 cores on the RP2350:

| Core | Primary Responsibility | Key Tasks |
| --- | --- | --- |
| **Core 0** | **Emulator Engine** | 6809 instruction execution, I/O polling (PS/2), device manager updates, and startup display list construction. |
| **Core 1** | **Video Signal Generator** | Running `core1_main` for the DVI driver. Handles timing and sending pixel data via HSTX. |

## Memory Configuration

The RP2350 has 520KB of SRAM. Rusty# CoCo allocates it as follows:

- **Heap**: **96KB** (using `embedded_alloc`).
- **RAM_DISK**: **64KB** (6809 address space).
- **Core 1 Stack**: **1024 bytes**.
- **Display Buffer**: Approximately **100KB** (256x192 RGB555 pixels used for rendering).
- **DVI Buffers**: Internal to the `pico-dvi-rs` driver.

## Display Scaling & Rendering

- **Native VDG Resolution**: 256x192 pixels.
- **Output Target**: VGA 640x480 @ 60Hz.
- **Vertical Scaling**: 2x (each VDG line is output twice to reach 384 active scanlines). This is handled natively in the DVI driver by defining stripes of height 2 and stride 0, which reuses the same VRAM scanline data twice consecutively without duplicating display list commands.
- **Horizontal Centering**: 192-pixel black margins on each side (640 - 256 = 384, 384 / 2 = 192).
- **Vertical Centering**: 48-pixel black margins on top and bottom (480 - 384 = 96, 96 / 2 = 48).
- **Display List Construction**: Built once at startup (rather than per-frame) since all framebuffer addresses are static. This reduces display list heap usage from **791KB (which exceeded RAM) to ~14KB**.

