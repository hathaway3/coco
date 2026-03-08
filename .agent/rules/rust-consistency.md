# Rust consistency rules

## Development workflow

* **No `std` in core**: The project is `no_std`. All core logic in `src/` must remain compatible with embedded targets.
* **Spinlocks**: Use `spin::Mutex` or `spin::RwLock` for synchronization instead of `std::sync`.
* **Allocations**: Use the `alloc` crate for dynamic memory management. Be mindful of heap usage on the RP2350.
* **Clippy & Fmt**: Ensure all code passes `cargo clippy` and is formatted with `cargo fmt`.

## Target: RP2350 (Pico 2)

* **Memory Efficiency**: The RP2350 has 520KB RAM, but much of it is used for the DVI framebuffer and emulator core. Avoid large stack allocations.
* **Multicore**: Core 1 is dedicated to DVI signal generation. Core 0 runs the emulator core and I/O polling.
* **Performance**: The 6809 emulator is timing-sensitive. Avoid heavy computations in the main execution loop.

## Debugging

* **Defmt**: Use `defmt::info!`, `defmt::error!`, etc., for logging on the embedded target.
* **Simulation**: Run `cargo test` on the host side for non-hardware-specific logic.
