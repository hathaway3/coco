# Rust consistency rules

## Development workflow

* **No `std` in core**: The project is `no_std`. All core logic in `src/` must remain compatible with embedded targets. Use `cfg_attr(target_os = "none", no_std)` at the crate root.
* **Synchronization**: Use `spin::Mutex` or `spin::RwLock` for shared state. Avoid `std::sync` entirely in `src/`.
* **Allocations**: Use the `alloc` crate and `Box`, `Vec`, `Arc` for dynamic memory. Be extremely mindful of heap usage on the RP2350 (96KB limited heap).
* **Target Gating**: Use `#[cfg(target_os = "none")]` for RP2350-specific code and `#[cfg(not(target_os = "none"))]` for host-only features (like `minifb` or `std` logging).
* **Clippy & Fmt**: Ensure all code passes `cargo clippy` and is formatted with `cargo fmt`.

## Target: RP2350 (Pico 2)

* **Memory Efficiency**: The RP2350 has 520KB RAM. DVI framebuffer and emulator core consume most of it.
  * Avoid large stack allocations.
  * Prefer static buffers for large fixed structures (like `RAM_DISK`).
* **Multicore**: 
  * **Core 1**: Dedicated to DVI signal generation (very timing sensitive).
  * **Core 0**: Runs the emulator engine, I/O polling, and display list construction.
* **Performance**: The 6809 emulator is timing-sensitive. Avoid heavy computations or blocking I/O in the main execution loop on Core 0.

## Debugging & Logging

* **Defmt**: Use `defmt::info!`, `defmt::error!`, etc., for logging on the embedded target.
* **Log**: Use the `log` crate (mapped to `env_logger` or similar) for host-side debugging.
* **Simulation**: Run `cargo test` on the host side for non-hardware-specific logic.
