use crate::editor::Editor;
use crate::keycodes::Key;
use crate::terminal::window::Window;
use errno::{errno, Errno};
use libc::{
    atexit, c_char, c_void, ioctl, read, sscanf, tcgetattr, tcsetattr, termios, winsize, write,
    BRKINT, CS8, EAGAIN, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON, OPOST,
    STDIN_FILENO, STDOUT_FILENO, TCSAFLUSH, TIOCGWINSZ, VMIN, VTIME,
};
use std::char;
use std::ffi::CString;
use std::io::{stdout, Write};

pub struct EditorImpl {}

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

fn get_window_size_ioctl<'a>() -> Option<Window<'a>> {
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
            Some(Window::new(ws.ws_col.into(), ws.ws_row.into()).window_size_method("ioctl"))
        }
    }
}

fn get_window_size_cursor_pos<'a>() -> Option<Window<'a>> {
    if let Ok(12) = stdout().write(b"\x1b[999C\x1b[999B") {
        stdout().flush().unwrap();
        if let Ok(4) = stdout().write(b"\x1b[6n") {
            stdout().write_all(b"\r\n").unwrap();
            stdout().flush().unwrap();

            let mut buf = vec![0u8; 32];
            let mut i = 0;

            while i < buf.len() - 1 {
                unsafe {
                    if read(STDIN_FILENO, buf[i..].as_mut_ptr() as *mut c_void, 1) != 1 {
                        break;
                    }
                }
                if buf[i] == b'R' {
                    break;
                }
                i += 1;
            }
            buf[i] = b'\0';

            if buf[0] != b'\x1b' || buf[1] != b'[' {
                None
            } else {
                let mut rows = 0;
                let mut cols = 0;
                let format = CString::new("%d;%d").unwrap();
                unsafe {
                    if sscanf(
                        buf[2..].as_ptr() as *const c_char,
                        format.as_ptr(),
                        &mut rows,
                        &mut cols,
                    ) != 2
                    {
                        None
                    } else {
                        Some(Window::new(rows, cols).window_size_method("cursor"))
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

extern "C" fn disable_raw_mode() {
    unsafe {
        if tcsetattr(STDIN_FILENO, TCSAFLUSH, &ORIG_TERMIOS) == -1 {
            panic!("tcsetattr");
        }
    }

    let mut reset_output = format!(
        "{}{}",
        "\x1b[2J", // clear screen
        "\x1b[H"   // goto origin
    );
    let len = reset_output.len();
    reset_output.push('\0');
    unsafe {
        write(STDOUT_FILENO, reset_output.as_ptr() as *const c_void, len);
    }
}

fn read_key() -> Option<Key> {
    let mut buf = vec![0u8; 1];
    let character;

    unsafe {
        let bytes_read = read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1);
        if bytes_read == -1 && errno() != Errno(EAGAIN) {
            panic!("read");
        }

        character = char::from(buf[0]);

        if character == '\x1b' {
            let mut buf = vec![0u8; 3];

            if read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) == -1 {
                return Some(Key::Escape);
            }

            if read(STDIN_FILENO, buf[1..].as_mut_ptr() as *mut c_void, 1) == -1 {
                return Some(Key::Escape);
            }

            if buf[0] == b'[' {
                if buf[1] >= b'0' && buf[1] <= b'9' {
                    if read(STDIN_FILENO, buf[2..].as_mut_ptr() as *mut c_void, 1) != 1 {
                        return Some(Key::Escape);
                    }
                    if buf[2] == b'~' {
                        match buf[1] {
                            b'1' => return Some(Key::Home),
                            b'3' => return Some(Key::Delete),
                            b'4' => return Some(Key::End),
                            b'5' => return Some(Key::PageUp),
                            b'6' => return Some(Key::PageDown),
                            b'7' => return Some(Key::Home),
                            b'8' => return Some(Key::End),
                            _ => return Some(Key::Escape),
                        }
                    }
                } else {
                    match buf[1] {
                        b'A' => return Some(Key::ArrowUp),
                        b'B' => return Some(Key::ArrowDown),
                        b'C' => return Some(Key::ArrowRight),
                        b'D' => return Some(Key::ArrowLeft),
                        b'H' => return Some(Key::Home),
                        b'F' => return Some(Key::End),
                        _ => return Some(Key::Escape),
                    }
                }
            } else if buf[0] == b'O' {
                match buf[1] {
                    b'H' => return Some(Key::Home),
                    b'F' => return Some(Key::End),
                    _ => return Some(Key::Escape),
                }
            }
        }
    }

    match character {
        '\r' => Some(Key::Return),
        '\u{7f}' => Some(Key::Backspace),
        '\0' => None,
        _ => Some(Key::Other(character)),
    }
}

impl Editor for EditorImpl {
    fn enable_raw_mode(&self) {
        unsafe {
            if tcgetattr(STDIN_FILENO, &mut ORIG_TERMIOS) == -1 {
                panic!("tcgetattr");
            }
            atexit(disable_raw_mode);
            let mut raw = ORIG_TERMIOS;
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

    fn get_window_size(&self) -> Window<'_> {
        get_window_size_ioctl()
            .or_else(get_window_size_cursor_pos)
            .unwrap()
    }

    fn read_a_character(&self) -> Option<Key> {
        read_key()
    }
}
