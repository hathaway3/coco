#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;
#[cfg(target_os = "none")]
pub fn get_keyboard_input(_block: bool, _echo: bool) -> Option<u8> {
    None
}
