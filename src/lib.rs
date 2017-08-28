extern crate libc;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::{enable_raw_mode, get_window_size, process_keypress};
#[cfg(unix)]
extern crate errno;

#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{enable_raw_mode, get_window_size, process_keypress};

mod keycodes;
pub mod config;
pub mod row;
pub mod terminal;
