#![no_std]
#![allow(warnings)]

extern crate alloc;

// Alias hal so submodules can find it via crate::hal
pub use rp235x_hal as hal;

pub mod clock;
pub mod demo;
pub mod dvi;
pub mod link;
pub mod render;
pub mod scanlist;

// Re-exports
pub use dvi::*;

// ==========================================
// Globals moved from main.rs to support library usage
// ==========================================
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use render::Palette4bppFast;

pub struct DviInstWrapper(pub UnsafeCell<MaybeUninit<DviInst>>);
unsafe impl Sync for DviInstWrapper {}

pub static DVI_INST: DviInstWrapper = DviInstWrapper(UnsafeCell::new(MaybeUninit::uninit()));
pub static DVI_OUT: DviOut = DviOut::new();

const PALETTE: &[u32; 16] = &[
    0x000000, 0xffffff, 0x9d9d9d, 0xe06f8b, 0xbe2633, 0x493c2b, 0xa46422, 0xeb8931, 0xf7e26b,
    0xa3ce27, 0x44891a, 0x2f484e, 0x1b2632, 0x5784, 0x31a2f2, 0xb2dcef,
];

#[link_section = ".data"]
pub static PALETTE_4BPP: Palette4bppFast = Palette4bppFast::new(PALETTE);
