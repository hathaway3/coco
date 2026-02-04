@echo off
echo Building project...
cargo build --release --target thumbv8m.main-none-eabihf
if %errorlevel% neq 0 exit /b %errorlevel%

echo Converting to UF2...
set ELF_PATH=target\thumbv8m.main-none-eabihf\release\coco
if exist "%ELF_PATH%" (
    elf2uf2-rs "%ELF_PATH%" "coco.uf2"
    echo Success! coco.uf2 created.
) else (
    echo Error: ELF file not found at %ELF_PATH%
    exit /b 1
)
pause
