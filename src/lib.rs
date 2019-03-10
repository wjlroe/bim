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
pub mod debug_log;
pub mod highlight;
mod keycodes;
mod row;
mod status;
mod syntax;
mod terminal;

pub mod gui;
