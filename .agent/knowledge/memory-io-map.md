# Memory & I/O Map: TRS-80 Color Computer

This document provides a detailed mapping of the 6809 address space as implemented in the Rusty# CoCo emulator.

## Memory Map (0x0000 - 0xFFFF)

| Range | Name | Description |
| --- | --- | --- |
| `0x0000` - `0x7FFF` | **System RAM** | 32KB of standard RAM. |
| `0x8000` - `0x9FFF` | **Extended Basic ROM** | Typically contains the Extended Color Basic ROM. |
| `0xA000` - `0xBFFF` | **Basic ROM** | Typically contains the Color Basic ROM. |
| `0xC000` - `0xDFFF` | **Cartridge ROM** | Space for cartridge binaries. |
| `0xE000` - `0xFDFF` | **High RAM** | Final 8KB of RAM (if 64KB configured). |
| `0xFE00` - `0xFEFF` | **Reserved** | Reserved space before the I/O block. |
| `0xFF00` - `0xFFFF` | **I/O Device Block** | Hardware registers and interrupt vectors. |

> [!IMPORTANT]
> **Interrupt Vector Remapping**: The emulator remaps addresses `0xFFE0..=0xFFFF` to `0xBFE0..=0xBFFF` in the internal `raw_ram` buffer (offset by `-0x4000`). This is a critical "gotcha" for vector management.

## I/O Device Registers

| Start | End | Device | Module |
| --- | --- | --- | --- |
| `0xFF00` | `0xFF1F` | **PIA 0** | [pia.rs](file:///C:/Users/kg4kn/coco/src/pia.rs) |
| `0xFF20` | `0xFF3F` | **PIA 1** | [pia.rs](file:///C:/Users/kg4kn/coco/src/pia.rs) |
| `0xFF40` | `0xFF5F` | Reserved / ACIA? | [acia.rs](file:///C:/Users/kg4kn/coco/src/acia.rs) |
| `0xFFC0` | `0xFFDF` | **SAM** | [sam.rs](file:///C:/Users/kg4kn/coco/src/sam.rs) |

### PIA 0 ($FF00) - Peripheral Interface Adapter 1

- **PA0..PA7**: Keyboard Row Strobe / Joystick Comparator (Bit 7).
- **PB0..PB7**: Keyboard Column Selection.
- **CA1**: HSync (Horizontal Sync interrupt).
- **CB1**: VSync (Vertical Sync interrupt / FIRQ).

### PIA 1 ($FF20) - Peripheral Interface Adapter 2

- **PA0..PA1**: RS-232 / DAC (Sound).
- **PA2..PA7**: VDG Mode Controls.
- **CB1**: Cartridge Detect (FIRQ).

### SAM ($FFC0) - Synchronous Address Multiplexer

The SAM handles memory layout and paging. Writes to specific addresses in the `0xFFC0..0xFFDF` range toggle internal state bits:

- **0xFFC0..0xFFC5**: VDG Mode Bits (V0, V1, V2).
- **0xFFC6..0xFFD3**: Display RAM Address Bits (F0..F6).
- **0xFFD4..0xFFD5**: Page 1 Selection.
- **0xFFD6..0xFFD9**: Clock speed control (0.89MHz vs 1.78MHz).
- **0xFFDA..0xFFDF**: Memory size/paging mode.
