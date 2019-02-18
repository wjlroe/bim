use crate::editor::Editor;
use crate::keycodes::Key;
use crate::terminal::Terminal;
use libc::atexit;
use std::char;
use std::mem::zeroed;
use std::ptr;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{DWORD, LPDWORD};
use winapi::um::consoleapi::{
    GetConsoleMode, ReadConsoleInputA, SetConsoleMode, WriteConsoleA,
};
use winapi::um::processenv::GetStdHandle;
use winapi::um::synchapi::WaitForSingleObjectEx;
use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, WAIT_OBJECT_0};
use winapi::um::wincon::{
    GetConsoleScreenBufferInfo, INPUT_RECORD_Event, CONSOLE_SCREEN_BUFFER_INFO,
    COORD, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_WRAP_AT_EOL_OUTPUT, INPUT_RECORD, KEY_EVENT, LEFT_CTRL_PRESSED,
    SMALL_RECT,
};

const ENABLE_VIRTUAL_TERMINAL_PROCESSING: DWORD = 0x0004;
const DISABLE_NEWLINE_AUTO_RETURN: DWORD = 0x0008;

static mut ORIG_INPUT_CONSOLE_MODE: DWORD = 0;
static mut ORIG_OUTPUT_CONSOLE_MODE: DWORD = 0;

// TODO: die! macro that clears the screen first
// TODO: rescue from panic, disabling raw mode

pub struct EditorImpl {}

extern "C" fn disable_raw_input_mode() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        SetConsoleMode(handle, ORIG_INPUT_CONSOLE_MODE);
    }

    let mut reset_output = format!(
        "{}{}",
        "\x1b[2J", // clear screen
        "\x1b[H"   // goto origin
    );
    let len: DWORD = reset_output.len() as DWORD;
    reset_output.push('\0');
    let mut bytes_written: DWORD = 0;
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        WriteConsoleA(
            handle,
            reset_output.as_ptr() as *const c_void,
            len,
            &mut bytes_written as LPDWORD,
            ptr::null_mut(),
        );
    }
}

extern "C" fn disable_raw_output_mode() {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        SetConsoleMode(handle, ORIG_OUTPUT_CONSOLE_MODE);
    }
}

impl Editor for EditorImpl {
    fn enable_raw_mode(&self) {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if GetConsoleMode(handle, &mut ORIG_INPUT_CONSOLE_MODE) != 0 {
                atexit(disable_raw_input_mode);
                let mut raw = ORIG_INPUT_CONSOLE_MODE;
                raw &= !(ENABLE_ECHO_INPUT
                    | ENABLE_LINE_INPUT
                    | ENABLE_PROCESSED_INPUT);
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
                let mut raw = ORIG_OUTPUT_CONSOLE_MODE;
                raw &= !(ENABLE_WRAP_AT_EOL_OUTPUT);
                raw |= DISABLE_NEWLINE_AUTO_RETURN
                    | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                if SetConsoleMode(handle, raw) == 0 {
                    panic!("setting console output mode failed!");
                }
            } else {
                panic!("getting output console didn't work");
            }
        }
    }

    fn read_a_character(&self) -> Option<Key> {
        use winapi::um::winuser::*;

        let mut character = None;
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            let waited = WaitForSingleObjectEx(handle, 1000, 1);
            if waited == WAIT_OBJECT_0 {
                let empty_record = INPUT_RECORD {
                    EventType: 0,
                    Event: zeroed::<INPUT_RECORD_Event>(),
                };
                let mut input_records = [empty_record];
                let mut events_read = 0;
                if ReadConsoleInputA(
                    handle,
                    input_records.as_mut_ptr(),
                    1,
                    &mut events_read,
                ) != 0
                {
                    if events_read == 1
                        && input_records[0].EventType == KEY_EVENT
                    {
                        let record = input_records[0].Event.KeyEvent();
                        if record.bKeyDown == 1 {
                            character = match i32::from(record.wVirtualKeyCode)
                            {
                                VK_UP => Some(Key::ArrowUp),
                                VK_DOWN => Some(Key::ArrowDown),
                                VK_LEFT => Some(Key::ArrowLeft),
                                VK_RIGHT => Some(Key::ArrowRight),
                                VK_PRIOR => Some(Key::PageUp),
                                VK_NEXT => Some(Key::PageDown),
                                VK_HOME => Some(Key::Home),
                                VK_END => Some(Key::End),
                                VK_DELETE => Some(Key::Delete),
                                VK_BACK => Some(Key::Backspace),
                                VK_RETURN => Some(Key::Return),
                                VK_ESCAPE => Some(Key::Escape),
                                VK_CONTROL => Some(Key::Control(None)),
                                VK_INSERT => Some(Key::Escape),
                                VK_SHIFT => Some(Key::Escape),
                                VK_LSHIFT => Some(Key::Escape),
                                VK_RSHIFT => Some(Key::Escape),
                                VK_MENU => Some(Key::Escape),
                                VK_CAPITAL => Some(Key::Escape),
                                VK_PAUSE => Some(Key::Escape),
                                VK_CLEAR => Some(Key::Escape),
                                VK_LWIN => Some(Key::Escape),
                                VK_APPS => Some(Key::Escape),
                                VK_SLEEP => Some(Key::Escape),
                                VK_SCROLL => Some(Key::Escape),
                                VK_VOLUME_MUTE => Some(Key::Escape),
                                VK_VOLUME_DOWN => Some(Key::Escape),
                                VK_VOLUME_UP => Some(Key::Escape),
                                _ => {
                                    let unicode_char =
                                        u32::from(*record.uChar.UnicodeChar());
                                    if record.dwControlKeyState
                                        & LEFT_CTRL_PRESSED
                                        == LEFT_CTRL_PRESSED
                                    {
                                        char::from_u32(unicode_char)
                                            .map(|uc| Key::Control(Some(uc)))
                                    } else {
                                        char::from_u32(unicode_char)
                                            .map(Key::Other)
                                    }
                                }
                            };
                        }
                    }
                } else {
                    panic!("ReadConsoleInputA failed");
                }
            }
        }

        character
    }

    fn get_window_size(&self) -> Terminal {
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut info = CONSOLE_SCREEN_BUFFER_INFO {
                dwSize: COORD { X: 0, Y: 0 },
                dwCursorPosition: COORD { X: 0, Y: 0 },
                dwMaximumWindowSize: COORD { X: 0, Y: 0 },
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
                Terminal::new(x.into(), y.into())
            } else {
                panic!("GetConsoleScreenBufferInfo failed to get window size!");
            }
        }
    }
}
