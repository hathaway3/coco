@echo off
where rustup >nul 2>nul
if %errorlevel% neq 0 (
    echo Rust not found. Please install Rust from https://rustup.rs/
    exit /b 1
)

echo Adding thumbv8m.main-none-eabihf target...
rustup target add thumbv8m.main-none-eabihf

echo Installing helpful tools...
cargo install elf2uf2-rs --locked

echo Setup complete! You can now run build_firmware.bat
pause
