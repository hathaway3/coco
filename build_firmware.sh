#!/bin/bash
set -e

echo "Building project..."

# Ensure cargo is available
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        echo "Cargo not found in PATH. Sourcing $HOME/.cargo/env..."
        source "$HOME/.cargo/env"
    else
        echo "Error: Cargo not found and $HOME/.cargo/env does not exist."
        echo "Please run ./setup_mac_linux.sh first."
        exit 1
    fi
fi
cargo build --release --target thumbv8m.main-none-eabihf

echo "Converting to UF2..."
ELF_PATH="target/thumbv8m.main-none-eabihf/release/coco"

if [ -f "$ELF_PATH" ]; then
    if command -v picotool &> /dev/null; then
        picotool uf2 convert "$ELF_PATH" -t elf "coco.uf2" --family rp2350-arm-s
    else
        echo "Error: picotool not found in PATH."
        echo "RP2350 (Pico 2) requires picotool for UF2 conversion (--family rp2350-arm-s); elf2uf2-rs only emits RP2040 UF2s."
        echo "Install picotool (e.g., 'brew install picotool' on macOS) and re-run."
        exit 1
    fi
    echo "Success! coco.uf2 created."
else
    echo "Error: ELF file not found at $ELF_PATH"
    exit 1
fi
