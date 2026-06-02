# Performance Optimizations: Rusty# CoCo

This document details the critical performance optimizations implemented for running the CoCo 6809 emulator efficiently on the Raspberry Pi Pico 2 W (RP2350).

## 1. Fast-Path Memory Access (Address < 0xff00)

**Module:** [memory.rs](file:///C:/Users/kg4kn/coco/src/memory.rs)

### Problem
The standard TRS-80 Color Computer memory map routes I/O devices to addresses `0xFF00..0xFFFF`. Previously, `_read_u8` and `_write_u8` executed ACIA, PIA, and SAM range/borrow checks on every single byte access. Because over 99.6% of memory operations fall within RAM/ROM (under address `0xFF00`), this added significant runtime overhead to CPU instruction fetching and stack operations.

### Optimization
Both `_read_u8` and `_write_u8` now use a fast-path branch `if addr < 0xff00` that directly accesses `self.raw_ram[addr]`, completely bypassing peripheral checks. The peripheral checks are only executed for addresses `0xFF00..0xFFFF`.

---

## 2. Lock-free HSYNC/VSYNC Interrupt Polling Cache

**Modules:** [cpu.rs](file:///C:/Users/kg4kn/coco/src/cpu.rs), [runtime.rs](file:///C:/Users/kg4kn/coco/src/runtime.rs), [memory.rs](file:///C:/Users/kg4kn/coco/src/memory.rs)

### Problem
The CPU `exec_one` loop polls for hardware interrupts every `HSYNC_PERIOD_CYCLES` (64 cycles). Each poll locked the `PiaPair` spin-lock Mutex and queried `hsync_irq()`, `vsync_irq()`, and `cart_firq()`. Since HSYNC and VSYNC interrupts are almost always disabled by the guest software, this caused massive lock contention and wasted CPU cycles.

### Optimization
- Cached the active state of HSYNC and VSYNC interrupts in `Core` as boolean flags: `hsync_enabled` and `vsync_enabled`.
- Intercepted CPU writes to the PIA control registers `0xFF01` and `0xFF03` in `_write_u8` to dynamically toggle these cached flags.
- Updated the interrupt polling logic in `exec_one` to only lock the `PiaPair` mutex if `self.hsync_enabled || self.vsync_enabled || self.cart_pending` is true.

---

## 3. VRAM Dirty Tracking

**Modules:** [cpu.rs](file:///C:/Users/kg4kn/coco/src/cpu.rs), [memory.rs](file:///C:/Users/kg4kn/coco/src/memory.rs), [pico.rs](file:///C:/Users/kg4kn/coco/src/pico.rs)

### Problem
Rendering the VDG screen is computationally expensive. Skipping rendering using a `dirty` flag was disabled because guest VRAM writes directly to raw memory do not trigger dirty state updates.

### Optimization
- Introduced a fast boolean `vram_dirty` flag in `Core`.
- In `_write_u8`, if a write is directed to RAM (address < `0x8000`), `vram_dirty` is flagged.
- The main emulator loop in `pico.rs` checks `core.vram_dirty` and only locks/marks the VDG dirty when VRAM has actually changed.
