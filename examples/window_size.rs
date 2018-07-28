#[cfg(windows)]
extern crate winapi;

extern crate libc;

#[cfg(unix)]
fn main() {
    use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};

    let mut ws = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        if ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) != -1 {
            println!(
                "ioctl win size. rows: {}, cols: {}",
                ws.ws_row, ws.ws_col
            );
        }
    }
}

#[cfg(not(unix))]
fn main() {
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::{
        GetConsoleScreenBufferInfo, CONSOLE_SCREEN_BUFFER_INFO, COORD,
        SMALL_RECT,
    };

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut info = CONSOLE_SCREEN_BUFFER_INFO {
            dwSize: COORD {
                X: 0,
                Y: 0,
            },
            dwCursorPosition: COORD {
                X: 0,
                Y: 0,
            },
            dwMaximumWindowSize: COORD {
                X: 0,
                Y: 0,
            },
            wAttributes: 0,
            srWindow: SMALL_RECT {
                Left: 0,
                Top: 0,
                Right: 0,
                Bottom: 0,
            },
        };
        if GetConsoleScreenBufferInfo(handle, &mut info) != 0 {
            let x = info.srWindow.Right - info.srWindow.Left + 1;
            let y = info.srWindow.Bottom - info.srWindow.Top + 1;
            println!("win32. rows: {}, cols: {}", y, x);
        }
    }
}
