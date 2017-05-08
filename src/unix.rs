use errno::{Errno, errno};
use keycodes::ctrl_key;
use libc::{BRKINT, CS8, EAGAIN, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG,
           ISTRIP, IXON, OPOST, STDIN_FILENO, TCSAFLUSH, VMIN, VTIME, atexit,
           c_void, isprint, read, tcgetattr, tcsetattr, termios};
use std::char;

#[cfg(target_os = "linux")]
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

#[cfg(not(target_os = "linux"))]
static mut ORIG_TERMIOS: termios = termios {
    c_iflag: 0,
    c_oflag: 0,
    c_lflag: 0,
    c_cflag: 0,
    c_cc: [0; 20],
    c_ospeed: 0,
    c_ispeed: 0,
};

extern "C" fn disable_raw_mode() {
    unsafe {
        if tcsetattr(STDIN_FILENO, TCSAFLUSH, &ORIG_TERMIOS) == -1 {
            panic!("tcsetattr");
        }
    }
}

fn enable_raw_mode() {
    unsafe {
        if tcgetattr(STDIN_FILENO, &mut ORIG_TERMIOS) == -1 {
            panic!("tcgetattr");
        }
        atexit(disable_raw_mode);
        let mut raw = ORIG_TERMIOS.clone();
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !(OPOST);
        raw.c_cflag |= CS8;
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;

        if tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw) == -1 {
            panic!("tcsetattr");
        }
    }
}

pub fn run() {
    enable_raw_mode();

    unsafe {
        loop {
            let mut buf = vec![0u8; 1];
            if read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) ==
               -1 && errno() != Errno(EAGAIN) {
                panic!("read");
            }
            let c = char::from(buf[0]);
            if isprint(c as i32) != 0 {
                println!("{:?} ('{}')\r", c as i32, c);
            } else {
                println!("{:?}\r", c as i32);
            }
            if ctrl_key('q', buf[0] as u32) {
                break;
            }
        }
    }
}
