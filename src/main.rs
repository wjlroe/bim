extern crate libc;

use libc::{termios, tcgetattr, tcsetattr, STDIN_FILENO, ECHO, TCSAFLUSH, atexit};
use std::char;
use std::io::{self, Read};

static mut orig_termios: termios = termios {
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
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
    }
}

fn enable_raw_mode() {
    unsafe {
        tcgetattr(STDIN_FILENO, &mut orig_termios);
        atexit(disable_raw_mode);
        let mut raw = orig_termios.clone();
        raw.c_lflag &= !(ECHO);
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
    }
}

fn main() {
    enable_raw_mode();

    let mut stdin = io::stdin();
    for byte in stdin.bytes() {
        if let Ok(byte_in) = byte {
            if let Some('q') = char::from_u32(byte_in as u32) {
                break;
            }
        }
    }
}
