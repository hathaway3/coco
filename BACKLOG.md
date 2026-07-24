# Backlog

Known outstanding work, tracked here since GitHub Issues are disabled on this repo.

- **SD card ROM loading** — Dynamic ROM loading from an SD card on the RP2350 build. Currently ROMs are baked in via `include_bytes!` at compile time. (noted as "coming soon" in README.md)
- **SAM V1/V2 handling** — `src/vdg.rs:298` has a `TODO SAM V1/V2` marking unhandled SAM version differences.
- **Overflow warnings in the assembler** — `src/parse.rs:185` has a `TODO` to warn when parsed operations overflow.
