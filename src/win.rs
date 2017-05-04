use kernel32::{GetConsoleMode, GetStdHandle, ReadConsoleW, SetConsoleMode};
use libc::atexit;
use std::char;
use std::ptr;
use winapi::minwindef::{DWORD, LPDWORD, LPVOID};
use winapi::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
                     ENABLE_PROCESSED_INPUT};

const ENABLE_VIRTUAL_TERMINAL_PROCESSING: DWORD = 0x0004;
const DISABLE_NEWLINE_AUTO_RETURN: DWORD = 0x0008;

static mut ORIG_INPUT_CONSOLE_MODE: DWORD = 0;
static mut ORIG_OUTPUT_CONSOLE_MODE: DWORD = 0;

extern "C" fn disable_raw_input_mode() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        SetConsoleMode(handle, ORIG_INPUT_CONSOLE_MODE);
    }
}

extern "C" fn disable_raw_output_mode() {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        SetConsoleMode(handle, ORIG_OUTPUT_CONSOLE_MODE);
    }
}

fn enable_raw_mode() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if GetConsoleMode(handle, &mut ORIG_INPUT_CONSOLE_MODE) != 0 {
            atexit(disable_raw_input_mode);
            let mut raw = ORIG_INPUT_CONSOLE_MODE.clone();
            raw &= !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT |
                     ENABLE_PROCESSED_INPUT);
            if SetConsoleMode(handle, raw) == 0 {
                panic!("setting console input mode failed!");
            }
        } else {
            panic!("getting input console didn't work");
        }
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if GetConsoleMode(handle, &mut ORIG_OUTPUT_CONSOLE_MODE) != 0 {
            atexit(disable_raw_output_mode);
            let mut raw = ORIG_OUTPUT_CONSOLE_MODE.clone();
            raw |= DISABLE_NEWLINE_AUTO_RETURN |
                   ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if SetConsoleMode(handle, raw) == 0 {
                panic!("setting console output mode failed!");
            }
        } else {
            panic!("getting output console didn't work");
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
                            println!("{:?} ('{}')\r", utf16[0], read_char);
                        } else {
                            println!("{:?}\r", utf16[0]);
                        }
                    }
                }
            } else {
                panic!("ReadConsoleW didn't work!");
            }
        }
    }
}

pub fn run() {
    enable_raw_mode();
    read_a_character();
}
