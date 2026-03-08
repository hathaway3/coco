---
description: How to debug the 6809 emulator and hardware simulation
---

1. **Host-Side Testing**:
   - Run `cargo test` to execute the unit tests in `src/cpu_test.rs` and `src/vdg_test.rs`.
   - Use `cargo run` if a host-based harness is available.
2. **Embedded Debugging**:
   - Check `defmt` output for software-level errors.
   - Use a debugger (e.g., probe-rs) to step through the Rust code on the RP2350.
3. **Emulator State**:
   - Use the built-in debug CLI if enabled (`config::debug()`).
   - Grep for `fault` or `panic` in the source to find error handling paths.
4. **Hardware issues**:
   - Check DVI signal generation on Core 1 if the screen is blank.
   - Verify I/O device registers in `devmgr.rs`.
