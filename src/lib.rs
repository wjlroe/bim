#[cfg(unix)]
extern crate libc;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::run;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::run;
