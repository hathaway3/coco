# 6809 Assembly Language Support: Rusty# CoCo

The Rusty# CoCo emulator includes a built-in 6809 assembler and parser, allowing for real-time assembly of source code into machine language.

## Assembly Process

The assembler (in [assembler.rs](file:///C:/Users/kg4kn/coco/src/assembler.rs)) uses a multi-pass strategy to resolve addresses and instruction sizes:

1. **Macro Expansion**: All macros are identified and expanded into the source stream.
2. **Label Identification**: Scans for label definitions and assigns estimated addresses.
3. **Instruction Sizing**: Instructions are analyzed. Sizing may change (e.g., a short branch becoming a long branch) as labels move.
4. **Label Resolution**: Labels are assigned final addresses once instruction sizes stabilize.
5. **Object Generation**: Binary machine code is generated and assigned to memory addresses.

## Assembler Directives

| Directive | Description | Example |
| --- | --- | --- |
| `ORG` | Set origin address for subsequent code. | `ORG $A000` |
| `EQU` | Define a constant or label value. | `REGS EQU $FF00` |
| `FCB` | Form Constant Byte(s). | `FCB $01, $02, $03` |
| `FDB` | Form Double Byte(s). | `FDB $1234, $5678` |
| `FCC` | Form Constant Character string. | `FCC /HELLO/` |
| `RMB` | Reserve Memory Bytes. | `BUFFER RMB 256` |
| `.MACRO` | Begin macro definition. | `.MACRO MYMAC` |
| `.ENDM` | End macro definition. | `.ENDM` |

## Operand Syntax & Addressing Modes

| Mode | Syntax | Description |
| --- | --- | --- |
| **Inherent** | `RTS` | No operand required. |
| **Immediate** | `LDA #$55` | Prefix with `#`. |
| **Direct** | `LDA <$00` | Prefix with `<` to force 8-bit DP addressing. |
| **Extended** | `LDA >$1234` | Prefix with `>` to force 16-bit addressing. |
| **Indexed** | `LDA ,X` | Comma followed by register (X, Y, U, S). |
| **Offset** | `LDA 5,X` | Constant offset relative to register. |
| **PC Relative** | `LDA 10,PCR` | Offset relative to Program Counter. |
| **Indirect** | `LDA [$1234]` | Bracketed operand for pointer indirection. |

## Expressions

The parser (in [parse.rs](file:///C:/Users/kg4kn/coco/src/parse.rs)) supports PEMDAS expressions:
- **Operators**: `+`, `-`, `*` (multiply), `/` (divide), `%` (mod), `^` (power).
- **Special Labels**: `*` refers to the current location counter (current address).
- **Base Prefix**: `$` for Hex, `%` for Binary, No prefix for Decimal.
