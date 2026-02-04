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
    # elf2uf2-rs might define 'elf2uf2-rs' binary or 'elf2uf2'
    if command -v elf2uf2-rs &> /dev/null; then
        elf2uf2-rs "$ELF_PATH" "coco.uf2"
    else 
        echo "Warning: elf2uf2-rs not found in PATH. Trying cargo run wrapper?"
        echo "Please ensure cargo bin is in PATH."
        exit 1
    fi
    echo "Success! coco.uf2 created."
else
    echo "Error: ELF file not found at $ELF_PATH"
    exit 1
fi
