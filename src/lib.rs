extern crate libc;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::run;
#[cfg(unix)]
extern crate errno;

#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::run;
