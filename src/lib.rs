pub mod editor;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod win;

#[cfg(unix)]
pub use crate::unix::EditorImpl;

#[cfg(windows)]
pub use crate::win::EditorImpl;

mod buffer;
mod commands;
pub mod config;
mod highlight;
mod keycodes;
mod row;
mod syntax;
mod terminal;
