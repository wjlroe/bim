#[cfg(unix)]
extern crate errno;
#[cfg(unix)]
extern crate libc;

#[cfg(unix)]
use errno::{errno, Errno};
#[cfg(unix)]
use libc::{atexit, c_void, read, tcgetattr, tcsetattr, termios, CS8, BRKINT,
           EAGAIN, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON,
           OPOST, STDIN_FILENO, TCSAFLUSH, VMIN, VTIME};

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

#[cfg(all(unix, not(target_os = "linux")))]
static mut ORIG_TERMIOS: termios = termios {
    c_iflag: 0,
    c_oflag: 0,
    c_lflag: 0,
    c_cflag: 0,
    c_cc: [0; 20],
    c_ospeed: 0,
    c_ispeed: 0,
};

#[cfg(unix)]
extern "C" fn disable_raw_mode() {
    unsafe {
        if tcsetattr(STDIN_FILENO, TCSAFLUSH, &ORIG_TERMIOS) == -1 {
            panic!("tcsetattr");
        }
    }
}

#[cfg(unix)]
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

#[cfg(unix)]
fn process_keypress() {
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
                return;
            }

            if read(STDIN_FILENO, buf[1..].as_mut_ptr() as *mut c_void, 1) == -1
            {
                return;
            }

            if buf[0] == b'[' {
                if buf[1] >= b'0' && buf[1] <= b'9' {
                    if read(
                        STDIN_FILENO,
                        buf[2..].as_mut_ptr() as *mut c_void,
                        1,
                    ) != 1
                    {
                        return;
                    }
                    if buf[2] == b'~' {
                        println!("x1b[{}~", buf[1]);
                    }
                } else {
                    println!("x1b[{}", buf[1]);
                }
            } else if buf[0] == b'O' {
                println!("x1bO{}", buf[1]);
            }
        } else {
            if 'q' == character {
                std::process::exit(0);
            }
            println!("{}", character.escape_default());
        }
    }
}

#[cfg(unix)]
fn main() {
    if cfg!(unix) {
        enable_raw_mode();
        loop {
            process_keypress();
        }
    }
}

#[cfg(not(unix))]
fn main() {}
