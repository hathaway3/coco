#![no_std]
#[macro_use]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

#[macro_use]
pub mod macros;
#[macro_use]
pub mod term;

pub mod acia;
pub mod assembler;
#[cfg(not(target_os = "none"))]
pub mod audio_test;
pub mod config;
pub mod cpu;
pub mod debug;
pub mod devmgr;
pub mod error;
pub mod hex;
pub mod input;
pub mod instructions;
pub mod memory;
pub mod obj;
pub mod parse;
pub mod pia;
// pub mod pico;
#[cfg(test)]
pub mod cpu_test;
pub mod program;
pub mod registers;
pub mod runtime;
pub mod sam;
#[cfg(not(target_os = "none"))]
pub mod sound;
#[cfg(not(target_os = "none"))]
pub mod test;
pub mod u8oru16;
pub mod vdg;
#[cfg(test)]
pub mod vdg_test;

// Re-export common types for external use (like main.rs) and internal modules via use super::*;
pub use crate::acia::Acia;
pub use crate::cpu::Core;
pub use crate::devmgr::DeviceManager;
pub use crate::error::{Error, ErrorKind};
// #[cfg(not(target_os = "none"))]
// impl From<std::io::Error> for Error {
//     fn from(e: std::io::Error) -> Self {
//         Error::new(ErrorKind::IO, None, e.to_string().as_str())
//     }
// }
pub use crate::pia::{Pia, Pia0, Pia1};
pub use crate::program::*;
pub use crate::sam::Sam;
pub use crate::vdg::{Color, Vdg, VdgMode};

// Re-export common types from alloc/core used by submodules via use super::*;
pub use alloc::boxed::Box;
pub use alloc::collections::{BTreeMap, BTreeMap as Map, VecDeque};
pub use alloc::format;
pub use alloc::rc::Rc;
pub use alloc::string::{String, ToString};
pub use alloc::sync::Arc;
pub use alloc::vec::Vec;

// Synchronization primitives for no_std
pub use spin::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use core::ffi::CStr;
pub use core::fmt;
pub use core::time::Duration;

pub(crate) use u8oru16::u8u16;

// Static buffers for embedded deployment
pub const SCREEN_DIM_X: usize = 256;
pub const SCREEN_DIM_Y: usize = 192;
pub static mut RAM_DISK: [u8; 0x10000] = [0u8; 0x10000];
pub static mut DISPLAY_BUFFER: [u16; SCREEN_DIM_X * SCREEN_DIM_Y] =
    [0x0000; SCREEN_DIM_X * SCREEN_DIM_Y];
