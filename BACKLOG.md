# Backlog

Known outstanding work, tracked here since GitHub Issues are disabled on this repo.

## Done

- ~~**SD card ROM loading**~~ — Already implemented in `src/pico.rs` (SPI0 + `embedded_sdmmc`, loads `COLOR.ROM`/`EXTENDED.ROM`/`CART.ROM` from the SD card root directory at boot). README updated to match; this was just stale documentation.
- ~~**SAM V1/V2 handling**~~ — The `TODO SAM V1/V2` comment in `src/vdg.rs` `render()` was stale: SAM's VDG mode bits are already fully resolved into `VdgMode` via `VdgMode::try_from_pia_and_sam` (`devmgr.rs`) before `render()` runs, and covered by `vdg_test.rs`. Comment removed.
- ~~**Overflow warnings in the assembler**~~ — `Div`/`Mod` in `src/parse.rs` `_eval_binary` previously had no bounds checking (unlike `Add`/`Sub`/`Star`/`Pow`, which already returned syntax errors on overflow), so an expression like `1/0` in assembly source would panic the assembler instead of producing a clean error. Now returns a `division by zero` / `modulo by zero` syntax error.
