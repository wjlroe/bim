use kernel32::{GetConsoleMode, GetStdHandle, ReadConsoleW, SetConsoleMode};
use libc::atexit;
use std::char;
use std::ptr;
use winapi::minwindef::{DWORD, LPDWORD, LPVOID};
use winapi::winbase::STD_INPUT_HANDLE;
use winapi::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
                     ENABLE_PROCESSED_INPUT};

static mut ORIG_CONSOLE_MODE: DWORD = 0;

extern "C" fn disable_raw_mode() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        SetConsoleMode(handle, ORIG_CONSOLE_MODE);
    }
}

fn enable_raw_mode() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if GetConsoleMode(handle, &mut ORIG_CONSOLE_MODE) != 0 {
            atexit(disable_raw_mode);
            let mut raw = ORIG_CONSOLE_MODE.clone();
            raw &= !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT |
                     ENABLE_PROCESSED_INPUT);
            if SetConsoleMode(handle, raw) != 0 {
                println!("set console mode");
            } else {
                println!("setting console mode failed!");
            }
        } else {
            println!("getting console didn't work");
        }
    }
}

fn read_a_character() {
    let mut running = true;

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        let mut utf16 = vec![0u16; 1];
        let mut chars_read: DWORD = 0;
        while running {
            if ReadConsoleW(handle,
                            utf16.as_mut_ptr() as LPVOID,
                            utf16.len() as u32,
                            &mut chars_read as LPDWORD,
                            ptr::null_mut()) != 0 {
                if chars_read > 0 {
                    // TODO: process each character if there are multiple?
                    let current_char = char::from_u32(utf16[0] as u32);
                    if let Some('q') = current_char {
                        running = false;
                    } else {
                        if let Some(read_char) = current_char {
                            println!("{:?} ('{}')", utf16[0], read_char);
                        } else {
                            println!("{:?}", utf16[0]);
                        }
                    }
                }
            } else {
                println!("ReadConsoleW didn't work!");
            }
        }
    }
}

pub fn run() {
    enable_raw_mode();
    read_a_character();
}
