use crate::vdg::{Color, Vdg, VdgMode, SCREEN_DIM_X, SCREEN_DIM_Y};
use crate::RAM_DISK;

/// Verify all Color::to_rgb555() values are correct RGB555 bit patterns.
/// RGB555 format: 0bRRRRR_GGGGG_BBBBB (bits 14-10: R, 9-5: G, 4-0: B)
#[test]
fn test_color_rgb555_bit_patterns() {
    // Black = all zeros
    assert_eq!(Color::Black.to_rgb555(), 0x0000);

    // Green = G channel only (bits 9-5 all set)
    let green = Color::Green.to_rgb555();
    assert_eq!(green & 0x7C00, 0, "Green should have no red");
    assert_eq!(green & 0x03E0, 0x03E0, "Green should have full green");
    assert_eq!(green & 0x001F, 0, "Green should have no blue");

    // Red = R channel only (bits 14-10 all set)
    let red = Color::Red.to_rgb555();
    assert_eq!(red & 0x7C00, 0x7C00, "Red should have full red");
    assert_eq!(red & 0x03E0, 0, "Red should have no green");
    assert_eq!(red & 0x001F, 0, "Red should have no blue");

    // Blue = B channel only (bits 4-0 all set)
    let blue = Color::Blue.to_rgb555();
    assert_eq!(blue & 0x7C00, 0, "Blue should have no red");
    assert_eq!(blue & 0x03E0, 0, "Blue should have no green");
    assert_eq!(blue & 0x001F, 0x001F, "Blue should have full blue");

    // Cyan = G + B
    let cyan = Color::Cyan.to_rgb555();
    assert_eq!(cyan & 0x7C00, 0, "Cyan should have no red");
    assert!(cyan & 0x03E0 != 0, "Cyan should have green");
    assert!(cyan & 0x001F != 0, "Cyan should have blue");

    // Magenta = R + B
    let magenta = Color::Magenta.to_rgb555();
    assert!(magenta & 0x7C00 != 0, "Magenta should have red");
    assert_eq!(magenta & 0x03E0, 0, "Magenta should have no green");
    assert!(magenta & 0x001F != 0, "Magenta should have blue");

    // Yellow = R + G
    let yellow = Color::Yellow.to_rgb555();
    assert!(yellow & 0x7C00 != 0, "Yellow should have red");
    assert!(yellow & 0x03E0 != 0, "Yellow should have green");
    assert_eq!(yellow & 0x001F, 0, "Yellow should have no blue");

    // Buff (white-ish) = all channels
    let buff = Color::Buff.to_rgb555();
    assert!(buff & 0x7C00 != 0, "Buff should have red");
    assert!(buff & 0x03E0 != 0, "Buff should have green");
    assert!(buff & 0x001F != 0, "Buff should have blue");

    // No value should exceed 15-bit range
    for code in 0..=8u8 {
        let color = Color::from_code(code);
        assert!(
            color.to_rgb555() <= 0x7FFF,
            "RGB555 value for {:?} exceeds 15-bit range",
            color
        );
    }
}

/// Verify Color round-trip: from_code maps correctly for all codes.
#[test]
fn test_color_from_code_roundtrip() {
    assert_eq!(Color::from_code(0), Color::Black);
    assert_eq!(Color::from_code(1), Color::Green);
    assert_eq!(Color::from_code(2), Color::Yellow);
    assert_eq!(Color::from_code(3), Color::Blue);
    assert_eq!(Color::from_code(4), Color::Red);
    assert_eq!(Color::from_code(5), Color::Buff);
    assert_eq!(Color::from_code(6), Color::Cyan);
    assert_eq!(Color::from_code(7), Color::Magenta);
    assert_eq!(Color::from_code(8), Color::Orange);
    // Out-of-range codes should default to Black
    assert_eq!(Color::from_code(9), Color::Black);
    assert_eq!(Color::from_code(255), Color::Black);
}

/// Verify VdgMode::try_from_pia_and_sam returns correct modes for known combinations.
#[test]
fn test_vdg_mode_selection() {
    // SG4: pia G/!A=0, GM0=0, sam=0
    assert_eq!(
        VdgMode::try_from_pia_and_sam(0b00000, 0),
        Some(VdgMode::SG4)
    );

    // SG6: pia G/!A=0, GM0=1, sam=0
    assert_eq!(
        VdgMode::try_from_pia_and_sam(0b00010, 0),
        Some(VdgMode::SG6)
    );

    // CG1: pia G/!A=1, GM0=0, sam=1
    assert_eq!(
        VdgMode::try_from_pia_and_sam(0b10000, 1),
        Some(VdgMode::CG1)
    );

    // RG1: pia G/!A=1, GM0=1, sam=1
    assert_eq!(
        VdgMode::try_from_pia_and_sam(0b10010, 1),
        Some(VdgMode::RG1)
    );

    // RG6: highest resolution, sam=6
    assert_eq!(
        VdgMode::try_from_pia_and_sam(0b11110, 6),
        Some(VdgMode::RG6)
    );

    // Invalid combination
    assert_eq!(VdgMode::try_from_pia_and_sam(0b11111, 7), None);
}

/// Verify VDG renders SG4 semigraphics blocks with correct colors.
/// In SG4, each VRAM byte defines a 4x6 pixel block:
///   bit 7 = 1: semigraphic block mode
///   bits 6-4: color (3-bit)
///   bits 3-0: which quadrants are lit
#[test]
fn test_render_sg4_semigraphic_block() {
    let mut vdg = Vdg::with_ram(0);
    vdg.set_mode(VdgMode::SG4);

    // Fill VRAM with a semigraphic byte: 0x8F
    // bit 7 = 1 (semigraphic), bits 6-4 = 000 (color green), bits 3-0 = 1111 (all quads lit)
    let vram_byte: u8 = 0x8F; // Green block, all quadrants filled
    unsafe {
        // Fill the first cell (byte 0) with the semigraphic block
        RAM_DISK[0] = vram_byte;
        // Fill rest of first row with 0 (space/blank)
        for i in 1..32 {
            RAM_DISK[i] = 0;
        }
    }

    let mut display = [0u16; SCREEN_DIM_X * SCREEN_DIM_Y];
    vdg.set_dirty();
    vdg.render(&mut display, false);

    // The first cell occupies 8x12 pixels at top-left
    // Check that pixels in the first 8x12 block are green (the lit quadrants)
    let green = Color::Green.to_rgb555();
    // SG4 semigraphic block: 4 quadrants in a 2x2 arrangement
    // Each quadrant is 4x3 pixels. Bits 0-3 control: TL, TR, BL, BR
    // All bits set means all 8x6 pixels should be the selected color (green)
    // Check top-left pixel
    assert_eq!(
        display[0], green,
        "Top-left pixel of green block should be green"
    );
    // Check other corner of the 8-pixel-wide cell (pixel x=7)
    assert_eq!(
        display[7], green,
        "Top-right pixel of green block should be green"
    );
}

/// Verify VDG renders ASCII text characters in SG4 mode.
/// In SG4, bytes 0x00-0x7F with bit 7=0 are character codes.
#[test]
fn test_render_sg4_text_char() {
    let mut vdg = Vdg::with_ram(0);
    vdg.set_mode(VdgMode::SG4);

    // Put ASCII 'A' (0x41 -> maps to internal code for A) in VRAM
    // CoCo charset: 0x41 ('A') will be rendered from the built-in font
    unsafe {
        RAM_DISK[0] = 0x41; // 'A'
        for i in 1..32 {
            RAM_DISK[i] = 0x20; // spaces
        }
    }

    let mut display = [0u16; SCREEN_DIM_X * SCREEN_DIM_Y];
    vdg.set_dirty();
    vdg.render(&mut display, false);

    // 'A' character should produce a mix of Green and Black pixels
    // in the 8x12 cell. Not all pixels should be black and not all green.
    let green = Color::Green.to_rgb555();
    let black = Color::Black.to_rgb555();

    let mut has_green = false;
    let mut has_black = false;
    for row in 0..12 {
        for col in 0..8 {
            let idx = row * SCREEN_DIM_X + col;
            if display[idx] == green {
                has_green = true;
            }
            if display[idx] == black {
                has_black = true;
            }
        }
    }
    assert!(
        has_green,
        "Character 'A' should have at least some green pixels (foreground)"
    );
    assert!(
        has_black,
        "Character 'A' should have at least some black pixels (background)"
    );
}
