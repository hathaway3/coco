@echo off
echo Building project...
cargo build --release --target thumbv8m.main-none-eabihf
if %errorlevel% neq 0 exit /b %errorlevel%

echo Converting to UF2...
set ELF_PATH=target\thumbv8m.main-none-eabihf\release\coco
if exist "%ELF_PATH%" (
    picotool uf2 convert "%ELF_PATH%" -t elf "coco.uf2" --family rp2350-arm-s
    if %errorlevel% neq 0 exit /b %errorlevel%
    echo Success! coco.uf2 created.
) else (
    echo Error: ELF file not found at %ELF_PATH%
    exit /b 1
)
pause
