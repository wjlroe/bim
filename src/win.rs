use kernel32::{GetConsoleMode, GetStdHandle, ReadConsoleInputA,
               SetConsoleMode, WaitForSingleObjectEx};
use keycodes::ctrl_key;
use libc::atexit;
use std::char;
use terminal::{clear_screen, refresh_screen};
use winapi::minwindef::DWORD;
use winapi::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, WAIT_OBJECT_0};
use winapi::wincon::{ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
                     ENABLE_PROCESSED_INPUT, INPUT_RECORD, KEY_EVENT};

const ENABLE_VIRTUAL_TERMINAL_PROCESSING: DWORD = 0x0004;
const DISABLE_NEWLINE_AUTO_RETURN: DWORD = 0x0008;

static mut ORIG_INPUT_CONSOLE_MODE: DWORD = 0;
static mut ORIG_OUTPUT_CONSOLE_MODE: DWORD = 0;

// TODO: die! macro that clears the screen first

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

fn process_keypress(key: char) {
    let char_num = key as u32;
    if ctrl_key('q', char_num) {
        clear_screen();
        ::std::process::exit(0);
    }
}

fn read_a_character() -> Option<char> {
    let mut character = None;
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        let waited = WaitForSingleObjectEx(handle, 1000, 1);
        if waited == WAIT_OBJECT_0 {
            let empty_record = INPUT_RECORD {
                EventType: 0,
                Event: [0; 4],
            };
            let mut input_records = [empty_record];
            let mut events_read = 0;
            if ReadConsoleInputA(handle,
                                 input_records.as_mut_ptr(),
                                 1,
                                 &mut events_read) != 0 {
                if events_read > 0 && input_records[0].EventType == KEY_EVENT {
                    let record = input_records[0].KeyEvent();
                    if record.bKeyDown == 0 {
                        let unicode_char = record.UnicodeChar as u32;
                        let read_char = char::from_u32(unicode_char);
                        character = read_char;
                    }
                }
            } else {
                panic!("ReadConsoleInputA failed");
            }
        }
    }

    character
}

pub fn run() {
    enable_raw_mode();
    loop {
        refresh_screen();
        if let Some(character) = read_a_character() {
            process_keypress(character);
        }
    }
}
