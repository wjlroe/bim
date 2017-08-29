extern crate libc;

pub mod editor;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::EditorImpl;
#[cfg(unix)]
extern crate errno;

#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::EditorImpl;

mod keycodes;
pub mod config;
pub mod row;
pub mod terminal;
