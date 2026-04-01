# Codebase Map: Rusty# CoCo

This document provides a high-level overview of the `src/` directory and the responsibilities of each module.

## Core Simulation

| Module | Description |
| --- | --- |
| [lib.rs](file:///C:/Users/kg4kn/coco/src/lib.rs) | Library entry point. Handles `no_std` configuration and exports core modules. |
| [cpu.rs](file:///C:/Users/kg4kn/coco/src/cpu.rs) | Defines the `Core` struct and the main instruction execution cycles. |
| [instructions.rs](file:///C:/Users/kg4kn/coco/src/instructions.rs) | Implementation of all 6809 opcodes and addressing modes. |
| [registers.rs](file:///C:/Users/kg4kn/coco/src/registers.rs) | 6809 register set (A, B, D, X, Y, U, S, PC, CC, DP). |
| [memory.rs](file:///C:/Users/kg4kn/coco/src/memory.rs) | Memory bus implementation. Handles reads/writes, ROM protection, and interrupt vector remapping. |
| [runtime.rs](file:///C:/Users/kg4kn/coco/src/runtime.rs) | Connects all components into a running emulator. Includes the main loop for the host build. |

## Hardware Emulation (Chips)

| Module | Description |
| --- | --- |
| [pia.rs](file:///C:/Users/kg4kn/coco/src/pia.rs) | Dual MC6821 Peripheral Interface Adapters. Handles keyboard, joystick, DAC, and VDG mode bits. |
| [vdg.rs](file:///C:/Users/kg4kn/coco/src/vdg.rs) | MC6847 Video Display Generator. Renders CoCo graphics modes (Text, SG, Semigraphics). |
| [sam.rs](file:///C:/Users/kg4kn/coco/src/sam.rs) | MC6883 Synchronous Address Multiplexer. Handles memory banking and addressing. |
| [acia.rs](file:///C:/Users/kg4kn/coco/src/acia.rs) | MC6850 ACIA. Serial communication support. |

## Infrastructure & Tooling

| Module | Description |
| --- | --- |
| [devmgr.rs](file:///C:/Users/kg4kn/coco/src/devmgr.rs) | Device Manager. Coordinates updates across PIA, VDG, and generic I/O devices. |
| [debug.rs](file:///C:/Users/kg4kn/coco/src/debug.rs) | Interactive 6809 debugger with disassembly and history. |
| [config.rs](file:///C:/Users/kg4kn/coco/src/config.rs) | YAML configuration loader (`coco.yaml`). |
| [error.rs](file:///C:/Users/kg4kn/coco/src/error.rs) | Project-wide error types. |

## Assembly & Code Loading

| Module | Description |
| --- | --- |
| [assembler.rs](file:///C:/Users/kg4kn/coco/src/assembler.rs) | Real-time 6809 assembler. |
| [parse.rs](file:///C:/Users/kg4kn/coco/src/parse.rs) | Parser for 6809 assembly language. |
| [obj.rs](file:///C:/Users/kg4kn/coco/src/obj.rs) / [hex.rs](file:///C:/Users/kg4kn/coco/src/hex.rs) | Object file and Motorola S-Record (Hex) support. |

## Target: RP2350 (Pico 2)

| Module | Description |
| --- | --- |
| [pico.rs](file:///C:/Users/kg4kn/coco/src/pico.rs) | Embedded entry point for RP2350. Handles DVI output, PS/2 keyboard, and multicore setup. |
| [memory.x](file:///C:/Users/kg4kn/coco/memory.x) | Linker script for the RP2350 target. |
