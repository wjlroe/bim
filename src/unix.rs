use libc::{ECHO, ICANON, STDIN_FILENO, TCSAFLUSH, atexit, tcgetattr,
           tcsetattr, termios};
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

pub fn run() {
    enable_raw_mode();

    let stdin = io::stdin();
    for byte in stdin.bytes() {
        if let Ok(byte_in) = byte {
            let maybe_char = char::from_u32(byte_in as u32);
            if let Some('q') = maybe_char {
                break;
            } else {
                if let Some(read_char) = maybe_char {
                    println!("{:?} ('{}')", byte_in, read_char);
                } else {
                    println!("{:?}", byte_in);
                }
            }
        }
    }
}
