# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A TRS-80 Color Computer 2 emulator in Rust targeting the Raspberry Pi Pico 2 (RP2350). Emulates the Motorola 6809E CPU, MC6847 VDG (video), dual MC6821 PIAs (keyboard/joystick/DAC), MC6883 SAM (memory banking), and MC6850 ACIA (serial). One crate produces both the embedded firmware (`src/pico.rs` is the `[[bin]]` entry) and a host-testable library.

**Important:** `.cargo/config.toml` sets the default build target to `thumbv8m.main-none-eabihf`, so plain `cargo build` builds the *embedded firmware*, and `cargo run` invokes `probe-rs run --chip RP2350` (requires a debug probe). The README's sections describing a host GUI emulator (minifb window, `--cart`, `--debug`, coco.yaml ROM loading at runtime) are legacy: `minifb`/`cpal`/`clap` are optional deps no longer referenced in `src/`, and the host `main()` only prints a pointer to the embedded build.

## Commands

```bash
# Build firmware (release ELF at target/thumbv8m.main-none-eabihf/release/coco)
cargo build --release

# Build + convert to coco.uf2 in repo root (drag onto Pico in BOOTSEL mode)
./build_firmware.sh          # Windows: build_firmware.bat

# Flash/run via debug probe (probe-rs)
cargo run --release

# Toolchain setup (rustup target, cargo-binutils, elf2uf2-rs)
./setup_mac_linux.sh         # Windows: setup_windows.bat

# Host-side tests — must override the default embedded target
cargo test --target aarch64-apple-darwin              # use your host triple (rustc -vV | grep host)
cargo test --target aarch64-apple-darwin <test_name>  # single test

# Lint/format (expected to pass per .agent rules)
cargo clippy
cargo fmt
```

Tests live in `src/cpu_test.rs` and `src/vdg_test.rs` (`#[cfg(test)]` modules). The test harness (`src/test.rs`) runs 6809 assembly programs and checks `;! <ident> = <value>` TestCriterion assertion comments embedded in them (e.g. `;! a = #$55`).

GitHub Issues are disabled on this repo; outstanding/completed work is tracked in `BACKLOG.md` instead. Check it for known issues before starting nontrivial work, and record what you did there when resolving a backlog item.

**Gotcha:** `cargo clippy --fix` can silently detach a `#[cfg(...)]` attribute from an `impl` block when auto-fixing lints like `new_without_default` (seen on `src/input/usb.rs`), breaking the embedded build with a duplicate method. Always run `cargo build --release` (embedded target) after `clippy --fix`, not just `cargo clippy`.

## Agent Knowledge Base

`.agent/` is the project's maintained knowledge base — read it before nontrivial work and **update it when making significant architectural changes**:

- `.agent/knowledge/codebase-map.md` — module-by-module map of `src/`
- `.agent/knowledge/memory-io-map.md` — CoCo memory layout, I/O registers, vector remapping
- `.agent/knowledge/pico-hardware.md` — RP2350 pins, multicore layout, display scaling
- `.agent/knowledge/6809-assembly.md` — built-in assembler directives and syntax
- `.agent/knowledge/performance-optimizations.md` — fast memory path, lock-free polling design
- `.agent/knowledge/drivewire-protocol.md` — DriveWire 4 protocol reference (fully supported via bridged ACIA)
- `.agent/rules/rust-consistency.md` — coding standards for no_std, synchronization, and target gating
- `.agent/rules/codebase-knowledge.md` — hardware gotchas and context-efficiency rules
- `.agent/workflows/` — step-by-step guides for adding opcodes and debugging the emulator

### Context-efficiency rules

1. **Grep Before View**: Before reading large files (e.g., `instructions.rs`, `assembler.rs`), grep to find the specific opcode or function you need.
2. **Use Outlines**: Always check module structure on new files over 200 lines before reading code.
3. **Log Priority**: When debugging, read terminal output or system logs first rather than guessing state.

## Architecture

### Dual-target no_std design

- The crate is `no_std` (`#![cfg_attr(not(test), no_std)]` in lib.rs). Gate RP2350-only code with `#[cfg(target_os = "none")]` and host-only code with `#[cfg(not(target_os = "none"))]`.
- Use `spin::Mutex`/`spin::RwLock` for shared state — never `std::sync` in `src/`.
- Dynamic memory comes from the `alloc` crate (`Box`, `Vec`, `Arc`). The device heap is only 96KB (`HEAP_SIZE` in `src/pico.rs`), so be frugal; prefer static buffers for large fixed structures (e.g. `RAM_DISK`).
- `defmt::info!`/`error!` for embedded logging (RTT); `DEFMT_LOG=debug` is set in `.cargo/config.toml`.

### Execution model

- **Core 0** runs the emulator loop, I/O polling, and display-list construction; **Core 1** is dedicated to DVI signal generation (timing-critical — keep heavy work off it). Multicore setup and the embedded entry point are in `src/pico.rs`; DVI comes from the vendored `vendor/pico-dvi-rs`.
- The CPU uses a direct-execution model: instructions mutate `Core` state in place (no per-instruction `Outcome` copying or `Vec` allocations). `src/cpu.rs` holds `Core` and the execution cycle; `src/instructions.rs` (~1300 lines) implements all opcodes/addressing modes — grep for the specific opcode before reading it whole.
- `src/runtime.rs` ties components together; `src/devmgr.rs` coordinates PIA/VDG/device updates; `src/memory.rs` is the bus.
- Assembly pipeline: `src/parse.rs` → `src/assembler.rs` (the emulator assembles and runs `.asm` files directly) with `src/obj.rs`/`src/hex.rs` for object/S-record formats.

### ROM loading

ROMs are loaded at boot from an SD card over SPI0. The firmware looks for:
- `COLOR.ROM` at `0xA000` (Color BASIC)
- `EXTENDED.ROM` at `0x8000` (Extended Color BASIC)
- `CART.ROM` at `0xC000` (cartridge)

If no SD card is found, a placeholder test ROM is used.

### Hardware gotchas (details in `.agent/rules/codebase-knowledge.md`)

- Interrupt vectors `0xFFE0..=0xFFFF` are remapped to `0xBFE0..=0xBFFF` in `raw_ram` (`src/memory.rs`).
- Addresses below `0xFF00` take a fast path in `_read_u8`/`_write_u8` that bypasses peripheral checks.
- `Pia0` and `Pia1` hold references to each other — watch lock ordering to avoid deadlock (`src/pia.rs`).
- VDG rendering is skipped unless `core.vram_dirty` is set; interrupt polling is skipped when HSync/VSync interrupts are disabled.
- PS/2 keyboard uses PIO on GPIO 28 (data) / 29 (clock); DVI occupies GPIO 12–19. USB Keyboard input and DriveWire 4 are bridged over USB CDC ACM Serial (self-sensing).
- Analog joystick is emulated via ADC0 on GPIO 26 (X-axis), ADC1 on GPIO 27 (Y-axis), GPIO 2 (Button 1), and GPIO 3 (Button 2).
- Hardware audio is output via PWM0 on GPIO 0.
- Emulator execution speed is throttled to exactly 0.895 MHz using the RP2350 microsecond timer.
- Memory budget on the RP2350 (520KB total): 64KB `RAM_DISK` + 98KB display buffer + 96KB heap, leaving roughly 170–190KB for stacks and DVI buffers. Avoid large stack allocations.
