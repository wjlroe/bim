use errno::{Errno, errno};
use keycodes::ctrl_key;
use libc::{BRKINT, CS8, EAGAIN, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG,
           ISTRIP, IXON, OPOST, STDIN_FILENO, STDOUT_FILENO, TCSAFLUSH,
           TIOCGWINSZ, VMIN, VTIME, atexit, c_void, ioctl, read, tcgetattr,
           tcsetattr, termios, winsize};
use std::char;
use std::io::{Write, stdout};
use terminal::{Terminal, clear_screen, refresh_screen};

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

fn get_window_size_ioctl() -> Option<Terminal> {
    let mut ws = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        if ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) == -1 || ws.ws_col == 0 {
            None
        } else {
            Some(Terminal::new(ws.ws_col as i32, ws.ws_row as i32))
        }
    }
}

fn get_window_size_cursor_pos() -> Option<Terminal> {
    if let Ok(12) = stdout().write(b"\x1b[999C\x1b[999B") {
        stdout().flush().unwrap();
        if let Ok(4) = stdout().write(b"\x1b[6n") {
            stdout().flush().unwrap();
            print!("\r\n");
            None
        } else {
            None
        }
    } else {
        None
    }
}

fn get_window_size() -> Option<Terminal> {
    get_window_size_ioctl().or_else(get_window_size_cursor_pos)
}

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

fn read_key() -> char {
    let mut buf = vec![0u8; 1];
    unsafe {
        if read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) == -1 &&
           errno() != Errno(EAGAIN) {
            panic!("read");
        }
    }
    char::from(buf[0])
}

fn process_keypress() {
    let c = read_key();

    if ctrl_key('q', c as u32) {
        clear_screen();
        ::std::process::exit(0);
    }
}

pub fn run() {
    enable_raw_mode();
    if let Some(terminal) = get_window_size() {
        loop {
            refresh_screen(&terminal);
            process_keypress();
        }
    } else {
        panic!("get_window_size didn't work");
    }
}
