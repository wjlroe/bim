extern crate libc;

use libc::{ECHO, ICANON, STDIN_FILENO, TCSAFLUSH, atexit, tcgetattr, tcsetattr,
           termios};
use std::char;
use std::io::{self, Read};

static mut ORIG_TERMIOS: termios = termios {
    c_iflag: 0,
    c_oflag: 0,
    c_lflag: 0,
    c_line: 0,
    c_cflag: 0,
    c_cc: [0; 32],
    c_ospeed: 0,
    c_ispeed: 0,
};

extern "C" fn disable_raw_mode() {
    unsafe {
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &ORIG_TERMIOS);
    }
}

fn enable_raw_mode() {
    unsafe {
        tcgetattr(STDIN_FILENO, &mut ORIG_TERMIOS);
        atexit(disable_raw_mode);
        let mut raw = ORIG_TERMIOS.clone();
        raw.c_lflag &= !(ECHO | ICANON);
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
    }
}

fn main() {
    enable_raw_mode();

    let stdin = io::stdin();
    for byte in stdin.bytes() {
        if let Ok(byte_in) = byte {
            if let Some('q') = char::from_u32(byte_in as u32) {
                break;
            }
        }
    }
}
