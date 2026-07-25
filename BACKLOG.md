# Backlog

Known outstanding work, tracked here since GitHub Issues are disabled on this repo.

## Outstanding

### Memory Efficiency

- **`format!()` allocations in instruction disassembly** — `src/runtime.rs:321-573` allocates multiple `String`s via `format!()` on every call to the disassembly path. Could implement `Display` traits to write directly to a formatter buffer, eliminating per-instruction heap allocations. Deferred: debug-only path (`help_humans()`), low priority.

## Done

- ~~**`get_set_bits()` allocates a Vec for 8-bit data**~~ — Function does not exist in the codebase; stale entry removed.
- ~~**`HexRecordCollection` uses unbounded Vec for records**~~ — `HexRecordCollection` is never constructed (`from_str_iter`, `read_from_file`, `write_to_file` all commented out); dead code.
- ~~**`Breakpoint::new()` clones symbol strings**~~ — `Breakpoint::new` has zero callers; dead code.
- ~~**`collect()` for token list in assembler**~~ — The referenced `collect()` at `src/assembler.rs:278` is inside a fully commented-out block.
- ~~**`PC::to_string()` and `PPPostByte::to_string()` return heap Strings**~~ — Replaced with `Display` trait implementations (`TEPostByte::PostByte` and `PPPostByte::PostByte` newtype wrappers) in `src/instructions.rs`. Updated call site in `src/runtime.rs`.
- ~~**`static mut` for AUDIO_CALLBACK**~~ — Already converted to `AtomicPtr<fn(u8)>` with `set_audio_callback()` / `invoke_audio_callback()` wrappers. Made `invoke_audio_callback` safe (moved `unsafe` inside the function) and removed redundant `unsafe` blocks in `src/pia.rs` call sites.
- ~~**Arc cloning in DeviceManager getters**~~ — `get_vdg()`, `get_pia()`, `get_sam()` have zero external callers; all internal usage accesses fields directly. Dead code.
- ~~**`PPPostByte::make(&Vec<String>)` takes `&Vec<String>`**~~ — Changed to `&[String]` for idiomatic API in `src/instructions.rs`.
- ~~**`needless_range_loop` warnings in `src/vdg_test.rs`**~~ — Replaced `for i in 1..32 { ram[i] = val; }` loops with `ram[1..32].fill(val)` at three locations (lines 139, 179, 234).
- ~~**`get_audio_samples()` allocation churn**~~ — Removed dead code: `audio_buffer` field, `push_audio_sample()`, and `get_audio_samples()` from `src/pia.rs`. The buffer was never consumed; actual audio output uses `AUDIO_CALLBACK` directly. This eliminates a `VecDeque<f32>` (4KB) from `PiaPair` and the `Vec<f32>` allocation on every call.
- ~~**SD card ROM loading**~~ — Already implemented in `src/pico.rs` (SPI0 + `embedded_sdmmc`, loads `COLOR.ROM`/`EXTENDED.ROM`/`CART.ROM` from the SD card root directory at boot). README updated to match; this was just stale documentation.
- ~~**SAM V1/V2 handling**~~ — The `TODO SAM V1/V2` comment in `src/vdg.rs` `render()` was stale: SAM's VDG mode bits are already fully resolved into `VdgMode` via `VdgMode::try_from_pia_and_sam` (`devmgr.rs`) before `render()` runs, and covered by `vdg_test.rs`. Comment removed.
- ~~**Overflow warnings in the assembler**~~ — `Div`/`Mod` in `src/parse.rs` `_eval_binary` previously had no bounds checking (unlike `Add`/`Sub`/`Star`/`Pow`, which already returned syntax errors on overflow), so an expression like `1/0` in assembly source would panic the assembler instead of producing a clean error. Now returns a `division by zero` / `modulo by zero` syntax error.
- ~~**`cargo clippy` / `cargo fmt` cleanup**~~ — Fixed all 39 pre-existing clippy warnings across both the host and embedded (`thumbv8m.main-none-eabihf`) targets, and ran `cargo fmt` repo-wide. Note: `cargo clippy --fix`'s automatic `new_without_default` fix for `src/input/usb.rs` silently detached the `#[cfg(not(target_os = "none"))]` attribute from the existing `impl UsbKeyboard` block, which briefly broke the embedded build (duplicate `new()`, missing fields) until caught by `cargo build --release` and fixed by hand — worth re-checking cfg-gated impls after any future `clippy --fix` run.
