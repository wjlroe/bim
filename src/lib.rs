pub mod editor;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod win;

#[cfg(unix)]
pub use crate::unix::EditorImpl;

#[cfg(windows)]
pub use crate::win::EditorImpl;

pub mod buffer;
mod commands;
pub mod config;
mod cursor;
pub mod debug_log;
pub mod highlight;
mod input;
mod keycodes;
mod prompt;
mod row;
mod search;
mod status;
mod status_line;
mod syntax;
mod terminal;
pub mod utils;

pub mod gui;
