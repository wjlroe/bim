use libc::{BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP,
           IXON, OPOST, STDIN_FILENO, TCSAFLUSH, VMIN, VTIME, atexit, c_void,
           isprint, read, tcgetattr, tcsetattr, termios};
use std::char;

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
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !(OPOST);
        raw.c_cflag |= CS8;
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;

        tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
    }
}

pub fn run() {
    enable_raw_mode();

    unsafe {
        loop {
            let mut buf = vec![0u8; 1];
            read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1);
            let c = char::from(buf[0]);
            if isprint(c as i32) != 0 {
                println!("{:?} ('{}')\r", c, c);
            } else {
                println!("{:?}\r", c);
            }
            if c == 'q' {
                break;
            }
        }
    }
}
