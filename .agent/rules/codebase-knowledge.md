# Codebase Knowledge Base

Before starting any work on the `coco` emulator, **read the knowledge base**:

- **[.agent/knowledge/codebase-map.md](../knowledge/codebase-map.md)** — High-level map of the `src/` directory and module responsibilities.
- **[.agent/knowledge/memory-io-map.md](../knowledge/memory-io-map.md)** — CoCo 2 memory layout, I/O registers, and interrupt vector remapping.
- **[.agent/knowledge/pico-hardware.md](../knowledge/pico-hardware.md)** — RP2350 (Pico 2) pin mappings, multicore architecture, and display scaling.
- **[.agent/knowledge/6809-assembly.md](../knowledge/6809-assembly.md)** — Built-in assembler behavior, directives, and operand syntax.
- **[.agent/knowledge/drivewire-protocol.md](../knowledge/drivewire-protocol.md)** — DriveWire 4 protocol reference.

This document is the single source of truth for codebase patterns. **Update it** when making significant architectural changes.

## context-efficiency rules

1. **Grep Before View**: Before reading large files (e.g., `instructions.rs`, `assembler.rs`), use `grep_search` to find the specific opcode or function you need.
2. **Use Outlines**: Always call `view_file_outline` on new files over 200 lines before reading code.
3. **Log Priority**: When debugging, read terminal output or system logs first rather than guessing state.
4. **Rust Safety**: This is a `no_std` project. Avoid using `std` features unless guarded by `cfg(not(target_os = "none"))`.

## Hardware Gotchas

| Feature | Description | File |
| --- | --- | --- |
| **Interrupt Vectors** | Addresses `0xFFE0..=0xFFFF` are remapped to `0xBFE0..=0xBFFF` in `raw_ram`. | [memory.rs](../src/memory.rs) |
| **PIA Deadlock** | `Pia0` and `Pia1` hold references to each other; be careful with lock order. | [pia.rs](../src/pia.rs) |
| **DAC Mux** | DAC value is top 6 bits of `Pia1-A`; `Pia0` reads it to set the joy comparator flag. | [pia.rs](../src/pia.rs) |
| **Pico PS/2** | Uses GPIO 28 (Data) and 29 (Clock) via PIO; these are the last available pins. | [pico.rs](../src/pico.rs) |

## Hardware Error Mapping (DriveWire/OS-9)

| OS-9 Error | DW Code | Pico Cause |
| --- | --- | --- |
| `E$NotRdy` | 246 | WiFi Down / Remote Server Timeout |
| `E$WP`     | 242 | SD Card Write Protect / Remote Drive |
| `E$CRC`    | 243 | Checksum mismatch in READEX/WRITE |
| `E$Read`   | 244 | Hardware SPI / SD read failure |
| `E$Unit`   | 240 | Invalid drive number (not 0-3) |
