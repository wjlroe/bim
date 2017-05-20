use errno::{Errno, errno};
use keycodes::ctrl_key;
use libc::{BRKINT, CS8, EAGAIN, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG,
           ISTRIP, IXON, OPOST, STDIN_FILENO, STDOUT_FILENO, TCSAFLUSH,
           TIOCGWINSZ, VMIN, VTIME, atexit, c_char, c_void, ioctl, read,
           sscanf, tcgetattr, tcsetattr, termios, winsize};
use std::char;
use std::ffi::CString;
use std::io::{Write, stdout};
use std::process::exit;
use terminal::Terminal;

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
            stdout().write(b"\r\n").unwrap();
            stdout().flush().unwrap();

            let mut buf = vec![0u8; 32];
            let mut i = 0;

            while i < buf.len() - 1 {
                unsafe {
                    if read(STDIN_FILENO,
                            buf[i..].as_mut_ptr() as *mut c_void,
                            1) != 1 {
                        break;
                    }
                }
                if buf[i] == 'R' as u8 {
                    break;
                }
                i += 1;
            }
            buf[i] = '\0' as u8;

            if buf[0] != '\x1b' as u8 || buf[1] != '[' as u8 {
                None
            } else {
                let mut rows = 0;
                let mut cols = 0;
                let format = CString::new("%d;%d").unwrap();
                unsafe {
                    if sscanf(buf[2..].as_ptr() as *const c_char,
                              format.as_ptr(),
                              &mut rows,
                              &mut cols) != 2 {
                        None
                    } else {
                        Some(Terminal::new(rows, cols))
                    }
                }
            }
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
    let mut character;

    unsafe {
        if read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) == -1 &&
           errno() != Errno(EAGAIN) {
            panic!("read");
        }

        character = char::from(buf[0]);

        if character == '\x1b' {
            let mut buf = vec![0u8; 3];

            if read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) == -1 {
                return '\x1b';
            }

            if read(STDIN_FILENO, buf[1..].as_mut_ptr() as *mut c_void, 1) ==
               -1 {
                return '\x1b';
            }

            if buf[0] == b'[' {
                character = match buf[1] {
                    b'A' => 'w',
                    b'B' => 's',
                    b'C' => 'd',
                    b'D' => 'a',
                    _ => '\x1b',
                }
            }
        }
    }

    character
}

fn process_keypress(mut terminal: &mut Terminal) {
    let c = read_key();

    if ctrl_key('q', c as u32) {
        terminal.reset();
        exit(0);
    } else {
        match c {
            'w' | 'a' | 's' | 'd' => terminal.move_cursor(c),
            _ => {}
        }
    }
}

pub fn run() {
    enable_raw_mode();
    if let Some(mut terminal) = get_window_size() {
        loop {
            terminal.refresh();
            process_keypress(&mut terminal);
        }
    } else {
        panic!("get_window_size didn't work");
    }
}
