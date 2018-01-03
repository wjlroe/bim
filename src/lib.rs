#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate time;

pub mod editor;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::EditorImpl;
#[cfg(unix)]
extern crate errno;

#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::EditorImpl;

mod keycodes;
mod commands;
mod syntax;
mod highlight;
mod row;
pub mod config;
mod buffer;
mod terminal;
