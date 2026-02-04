#!/bin/bash
set -e

echo "Ensuring Rust is installed..."
if ! command -v rustup &> /dev/null; then
    echo "Rust not found. Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "Adding thumbv8m.main-none-eabihf target..."
rustup target add thumbv8m.main-none-eabihf

echo "Installing helpful tools..."
# Check if installed to save time? Cargo install checks version usually.
cargo install elf2uf2-rs --no-default-features --features "libusb" --locked || cargo install elf2uf2-rs --locked
# Note: elf2uf2-rs sometimes fails on libusb dependency on some systems. 
# We try to valid install command. 
# Actually, `cargo install elf2uf2-rs` usually works if libudev is there.
# Let's keep it simple.

echo "Setup complete! You can now run ./build_firmware.sh"
