#!/bin/bash
set -e

echo "Ensuring Rust is installed..."
if ! command -v rustup &> /dev/null; then
    echo "Rust not found. Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Detected Linux. Checking for required core dependencies (pkg-config, libudev)..."
    
    install_deps() {
        if command -v apt-get &> /dev/null; then
            echo "Installing dependencies using apt-get..."
            sudo apt-get update
            # libudev-dev is needed for libudev-sys, pkg-config for finding it
            sudo apt-get install -y pkg-config libudev-dev build-essential
        elif command -v dnf &> /dev/null; then
            echo "Installing dependencies using dnf..."
            # libudev-devel on Fedora/RHEL
            sudo dnf install -y pkgconf-pkg-config libudev-devel gcc
        elif command -v pacman &> /dev/null; then
            echo "Installing dependencies using pacman..."
            sudo pacman -S --noconfirm pkgconf base-devel
        else
            echo "WARNING: Could not detect a supported package manager (apt, dnf, pacman)."
            echo "Please ensure 'pkg-config' and 'libudev' development headers are installed manually."
        fi
    }

    if ! command -v pkg-config &> /dev/null; then
        echo "pkg-config not found. Installing..."
        install_deps
    else
        echo "pkg-config found. ensuring libudev dependencies are also present..."
        # We run install_deps anyway to ensure libudev-dev/headers are there, 
        # as checking for headers specifically is harder.
        # Users with sudo access will be prompted if needed.
        install_deps
    fi
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
