use crate::buffer::{Buffer, FileSaveStatus};
use crate::commands::{Cmd, MoveCursor, SearchDirection};
use crate::config::BIM_QUIT_TIMES;
use crate::cursor::CursorT;
use crate::debug_log::DebugLog;
use crate::editor::BIM_VERSION;
use crate::keycodes::{ctrl_key, Key};
use crate::options::Options;
use crate::status::Status;
use crate::terminal::buffer::TerminalBuffer;
use std::error::Error;
use std::io::{stdout, Write};
use std::process::exit;
use std::time::Duration;

const UI_ROWS: i32 = 2;
const BIM_DEBUG_LOG: &str = ".bim_debug";

pub struct Window<'a> {
    pub screen_cols: i32,
    pub screen_rows: i32,
    window_size_method: &'a str,
    rcursor_x: i32,
    buffer: Buffer<'a>,
    // TODO: Use a container instead of a buffer directly
    append_buffer: String,
    pub row_offset: i32,
    pub col_offset: i32,
    quit_times: i8,
    status: Option<Status>,
    pub debug_log: DebugLog<'a>,
    pub options: Options,
}

impl<'a> Window<'a> {
    pub fn new(screen_cols: i32, screen_rows: i32) -> Self {
        Self {
            screen_cols,
            screen_rows,
            window_size_method: "",
            rcursor_x: 0,
            buffer: Buffer::default(),
            append_buffer: String::new(),
            row_offset: 0,
            col_offset: 0,
            quit_times: BIM_QUIT_TIMES,
            status: None,
            debug_log: DebugLog::new(BIM_DEBUG_LOG),
            options: Options::default(),
        }
    }

    pub fn window_size_method(mut self, method: &'a str) -> Self {
        self.window_size_method = method;
        self
    }

    fn die(&mut self, message: &str) {
        self.reset();

        println!("Error: {}", message);
        exit(1);
    }

    fn draw_rows(&mut self) {
        let num_rows = self.buffer.num_lines() as i32;
        for i in 0..self.screen_rows {
            let file_row = i + self.row_offset;
            if file_row >= num_rows {
                if num_rows == 0 && i == self.screen_rows / 3 {
                    let mut welcome = format!("bim editor - version {}", BIM_VERSION);
                    welcome.truncate(self.screen_cols as usize);
                    let mut padding = (self.screen_cols - welcome.len() as i32) / 2;
                    if padding > 0 {
                        self.append_buffer.push_str("~");
                        padding -= 1;
                    }
                    // TODO: can we pad with spaces easier?
                    let padding_str = format!("{:1$}", "", padding as usize);
                    self.append_buffer.push_str(&padding_str);
                    self.append_buffer.push_str(&welcome);
                } else {
                    self.append_buffer.push_str("~");
                }
            } else if let Some(onscreen_row) = self.buffer.row_onscreen_text(
                file_row as usize,
                self.col_offset as usize,
                self.screen_cols as usize,
            ) {
                self.append_buffer.push_str(onscreen_row.as_str());
            }

            self.clear_line();

            self.append_buffer.push_str("\r\n");
        }
    }

    fn draw_status_bar(&mut self) {
        self.append_buffer.push_str("\x1b[7m");
        let filename = self
            .buffer
            .filename
            .clone()
            .unwrap_or_else(|| String::from("[No Name]"));
        let file_status = if self.buffer.is_dirty() {
            "(modified)"
        } else {
            ""
        };
        let filetype = self.buffer.get_filetype();

        let mut status = format!(
            "{0:.20} - {1} lines {2}",
            filename,
            self.buffer.num_lines(),
            file_status
        );
        let right_status = format!(
            "{} | {}/{}",
            filetype,
            self.buffer.cursor.text_row() + 1,
            self.buffer.num_lines()
        );
        status.truncate(self.screen_cols as usize);
        self.append_buffer.push_str(&status);
        let remaining = self.screen_cols - status.len() as i32 - right_status.len() as i32;
        for _ in 0..remaining {
            self.append_buffer.push(' ');
        }
        self.append_buffer.push_str(&right_status);
        self.append_buffer.push_str("\x1b[m");
        self.append_buffer.push_str("\r\n");
    }

    fn draw_message_bar(&mut self) {
        self.clear_line();
        if let Some(ref status) = self.status {
            if status.is_valid() {
                let mut msg = status.message.clone();
                msg.truncate(self.screen_cols as usize);
                self.append_buffer.push_str(&msg);
            } else {
                self.status = None;
            }
        }
    }

    fn goto_origin(&mut self) {
        self.append_buffer.push_str("\x1b[H");
    }

    fn clear_line(&mut self) {
        self.append_buffer.push_str("\x1b[K");
    }

    fn clear(&mut self) {
        self.append_buffer.push_str("\x1b[2J");
    }

    fn hide_cursor(&mut self) {
        self.append_buffer.push_str("\x1b[?25l");
    }

    fn show_cursor(&mut self) {
        self.append_buffer.push_str("\x1b[?25h");
    }

    fn reset_cursor(&mut self) {
        let ansi = format!(
            "\x1b[{};{}H",
            (self.buffer.cursor.text_row() - self.row_offset) + 1,
            (self.rcursor_x - self.col_offset) + 1
        );
        self.append_buffer.push_str(&ansi);
    }

    pub fn reset(&mut self) {
        self.clear();
        self.goto_origin();
        if let Err(err) = self.flush() {
            panic!("oh no! flush failed: {:?}", err);
        };
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        {
            let output = self.append_buffer.as_bytes();
            let output_size = output.len();
            let written_bytes = stdout().write(output)?;
            if written_bytes == output_size {
                stdout().flush()?;
            } else {
                let failed = "Failed to write all the output.";
                let err_desc = format!(
                    "{} output_size = {} written_bytes = {}",
                    failed, output_size, written_bytes
                );
                return Err(err_desc).map_err(|err| err.into());
            }
        }
        self.append_buffer.clear();
        Ok(())
    }

    fn scroll(&mut self) {
        self.rcursor_x = 0;
        if self.buffer.cursor.text_row() < self.buffer.num_lines() as i32 {
            self.rcursor_x = self.buffer.text_cursor_to_render(
                self.buffer.cursor.text_col(),
                self.buffer.cursor.text_row(),
            );
        }

        if self.buffer.cursor.text_row() < self.row_offset {
            self.row_offset = self.buffer.cursor.text_row();
        }

        if self.buffer.cursor.text_row() >= self.row_offset + self.screen_rows {
            self.row_offset = self.buffer.cursor.text_row() - self.screen_rows + 1;
        }

        if self.rcursor_x < self.col_offset {
            self.col_offset = self.rcursor_x;
        }

        if self.rcursor_x >= self.col_offset + self.screen_cols {
            self.col_offset = self.rcursor_x - self.screen_cols + 1;
        }
    }

    pub fn refresh(&mut self) {
        self.scroll();

        self.hide_cursor();
        self.goto_origin();

        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();

        self.reset_cursor();

        self.show_cursor();

        if let Err(err) = self.flush() {
            panic!("oh no! flush failed: {:?}", err);
        };
    }

    pub fn move_cursor(&mut self, movement: MoveCursor) {
        // FIXME: This is a duplicate of src/pane.rs:do_cursor_movement()
        // We need to use Pane in terminal to share that code
        use crate::commands::Direction::*;
        use crate::commands::MoveUnit::*;

        let page_size = self.screen_rows as usize;
        let num_lines = self.buffer.num_lines();

        match movement {
            MoveCursor {
                unit: Rows,
                direction: Up,
                amount,
            } => {
                self.buffer.cursor.change(|cursor| {
                    let max_amount = cursor.text_row();
                    let possible_amount = std::cmp::min(amount as i32, max_amount);
                    cursor.text_row -= possible_amount;
                });
            }
            MoveCursor {
                unit: Rows,
                direction: Down,
                amount,
            } => {
                self.buffer.cursor.change(|cursor| {
                    let max_movement = num_lines as i32 - 1 - cursor.text_row();
                    let possible_amount = std::cmp::min(amount as i32, max_movement);
                    cursor.text_row += possible_amount;
                });
            }
            MoveCursor {
                unit: Cols,
                direction: Left,
                amount,
            } => {
                let mut new_cursor = self.buffer.cursor.current();
                let mut left_amount = amount as i32;
                while left_amount > 0 {
                    if new_cursor.text_col != 0 {
                        new_cursor.text_col -= 1;
                    } else if new_cursor.text_row > 0 {
                        new_cursor.text_row -= 1;
                        new_cursor.text_col =
                            self.buffer.line_len(new_cursor.text_row).unwrap_or(0) as i32;
                    } else {
                        break;
                    }
                    left_amount -= 1;
                }
                self.buffer.cursor.change(|cursor| {
                    cursor.text_col = new_cursor.text_col();
                    cursor.text_row = new_cursor.text_row();
                });
            }
            MoveCursor {
                unit: Cols,
                direction: Right,
                amount,
            } => {
                let mut new_cursor = self.buffer.cursor.current();
                let mut right_amount = amount as i32;
                let num_lines = self.buffer.num_lines() as i32;
                while right_amount > 0 {
                    if let Some(row_size) = self.buffer.line_len(new_cursor.text_row) {
                        if new_cursor.text_col < row_size as i32 {
                            new_cursor.text_col += 1;
                        } else if new_cursor.text_col == row_size as i32
                            && new_cursor.text_row < num_lines - 1
                        {
                            new_cursor.text_row += 1;
                            new_cursor.text_col = 0;
                        } else {
                            break;
                        }
                        right_amount -= 1;
                    } else {
                        break;
                    }
                }
                self.buffer.cursor.change(|cursor| {
                    cursor.text_col = new_cursor.text_col();
                    cursor.text_row = new_cursor.text_row();
                });
            }
            MoveCursor {
                unit: Start,
                direction: Left,
                ..
            } => self.buffer.cursor.change(|cursor| cursor.text_col = 0),
            MoveCursor {
                unit: End,
                direction: Right,
                ..
            } => {
                let new_x = self
                    .buffer
                    .line_len(self.buffer.cursor.text_row())
                    .unwrap_or(0) as i32;
                self.buffer.cursor.change(|cursor| {
                    cursor.text_col = new_x;
                });
            }
            MoveCursor {
                unit: Pages,
                direction: Down,
                amount,
            } => {
                let amount = amount * page_size;
                self.move_cursor(MoveCursor::down(amount));
            }
            MoveCursor {
                unit: Pages,
                direction: Up,
                amount,
            } => {
                let amount = amount * page_size;
                self.move_cursor(MoveCursor::up(amount));
            }
            _ => {}
        }
        self.buffer.check_cursor();
    }

    fn insert_char(&mut self, character: char) {
        self.buffer.insert_char_at_cursor(character);
    }

    fn delete_char(&mut self) {
        self.buffer.delete_char_at_cursor();
    }

    fn insert_newline_and_return(&mut self) {
        self.buffer.insert_newline_and_return();
    }

    pub fn process_key(&mut self, key: Key) {
        if let Some(cmd) = self.key_to_cmd(key) {
            self.process_cmd(cmd);
        }
    }

    pub fn process_cmd(&mut self, cmd: Cmd) {
        use crate::commands::Cmd::*;

        match cmd {
            Move(move_cursor) => self.move_cursor(move_cursor),
            Quit => {
                if self.options.show_quit_warning()
                    && self.buffer.is_dirty()
                    && self.quit_times.is_positive()
                {
                    let quit_times = self.quit_times;
                    self.set_status_message(format!(
                        "{} {} {} {}",
                        "WARNING! File has unsaved changes.",
                        "Press Ctrl-Q",
                        quit_times,
                        "more times to quit."
                    ));
                    self.quit_times -= 1;
                    return;
                } else {
                    self.reset();
                    exit(0);
                }
            }
            DeleteCharBackward => self.delete_char(),
            DeleteCharForward => {
                self.move_cursor(MoveCursor::right(1));
                self.delete_char();
            }
            Linebreak => {
                self.insert_newline_and_return();
            }
            Save => self.save_file(),
            InsertChar(c) => self.insert_char(c),
            Search => {}
            CloneCursor => {}
            PrintInfo => {}
            Escape => {}
        }

        self.quit_times = BIM_QUIT_TIMES;
    }

    pub fn key_to_cmd(&self, key: Key) -> Option<Cmd> {
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("key press: {:?}", key));
        match key {
            Key::ArrowLeft => Some(Cmd::Move(MoveCursor::left(1))),
            Key::ArrowRight => Some(Cmd::Move(MoveCursor::right(1))),
            Key::ArrowUp => Some(Cmd::Move(MoveCursor::up(1))),
            Key::ArrowDown => Some(Cmd::Move(MoveCursor::down(1))),
            Key::PageUp => Some(Cmd::Move(MoveCursor::page_up(1))),
            Key::PageDown => Some(Cmd::Move(MoveCursor::page_down(1))),
            Key::Home => Some(Cmd::Move(MoveCursor::home())),
            Key::End => Some(Cmd::Move(MoveCursor::end())),
            Key::Delete => Some(Cmd::DeleteCharForward),
            Key::Backspace => Some(Cmd::DeleteCharBackward),
            Key::Return => Some(Cmd::Linebreak),
            Key::Escape => None,
            Key::Function(_) => None,
            Key::Control(None) => None,
            Key::Control(Some(c)) => {
                if ctrl_key('q', c as u32) {
                    Some(Cmd::Quit)
                } else if ctrl_key('f', c as u32) {
                    Some(Cmd::Search)
                } else if ctrl_key('h', c as u32) {
                    Some(Cmd::DeleteCharBackward)
                } else if ctrl_key('s', c as u32) {
                    Some(Cmd::Save)
                } else {
                    None
                }
            }
            Key::TypedChar => None, // this means non-specific typed character
            Key::Other(c) => {
                let _ = self.debug_log.debugln_timestamped(&format!(
                    "other key: {character}, {key_num:x}, {key_num} as u32",
                    character = c,
                    key_num = c as u32
                ));
                if ctrl_key('h', c as u32) {
                    Some(Cmd::DeleteCharBackward)
                } else if ctrl_key('q', c as u32) {
                    Some(Cmd::Quit)
                } else if ctrl_key('s', c as u32) {
                    Some(Cmd::Save)
                } else if ctrl_key('l', c as u32) {
                    None
                } else if ctrl_key('f', c as u32) {
                    Some(Cmd::Search)
                } else if !c.is_control() {
                    Some(Cmd::InsertChar(c))
                } else {
                    None
                }
            }
        }
    }

    pub fn clear_search_overlay(&mut self) {
        self.buffer.clear_search_overlay();
    }

    pub fn set_status_message(&mut self, message: String) {
        let status = Status::new_with_timeout(message, Duration::from_secs(5));
        self.status = Some(status);
    }

    pub fn open(&mut self, filename: &str) {
        if let Err(e) = self.buffer.open(filename) {
            self.die(e.description());
        };
    }

    pub fn has_filename(&self) -> bool {
        self.buffer.filename.is_some()
    }

    pub fn set_filename(&mut self, filename: String) {
        self.buffer.set_filename(filename);
    }

    pub fn init(&mut self) {
        self.start_debug();
        self.set_status_message(String::from(
            "HELP: Ctrl-S = save | Ctrl-Q = quit | Ctrl-F = find",
        ));

        self.screen_rows -= UI_ROWS;
    }

    fn start_debug(&self) {
        let _ = self.debug_log.start();
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("bim version {} starting", BIM_VERSION));
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("rows: {}", self.screen_rows));
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("cols: {}", self.screen_cols));
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("window size method: {}", self.window_size_method));
    }

    pub fn log_debug(&self) {
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("rows: {}", self.screen_rows + UI_ROWS));
        let _ = self
            .debug_log
            .debugln_timestamped(&format!("cols: {}", self.screen_cols));
    }

    pub fn save_file(&mut self) {
        match self.buffer.save_file() {
            Ok(FileSaveStatus::Saved(bytes_saved)) => {
                self.set_status_message(format!("{} bytes written to disk", bytes_saved));
            }
            Ok(_) => {} // FIXME: handle NoFilename etc.
            Err(err) => self.set_status_message(format!("Can't save! Error: {:?}", err)),
        }
    }

    pub fn search_for(
        &mut self,
        last_match: Option<(usize, usize)>,
        direction: SearchDirection,
        needle: &str,
    ) -> Option<(usize, usize)> {
        let _ = self.debug_log.debugln_timestamped(&format!(
            "search_for: '{}', direction: {}",
            needle, direction
        ));
        let found_match = self.buffer.search_for(last_match, direction, needle);
        if found_match.is_some() {
            self.row_offset = self.buffer.num_lines() as i32;
        }
        found_match
    }

    pub fn save_cursor(&mut self) {
        self.buffer.cursor.save_cursor();
    }

    pub fn restore_cursor(&mut self) {
        self.buffer.cursor.restore_saved();
    }
}

#[test]
fn test_backspace_to_join_lines() {
    let mut terminal = Window::new(10, 10);

    terminal.buffer.append_row("this is the first line. \r\n");
    terminal.buffer.append_row("this is second line.\r\n");
    assert_eq!(0, terminal.buffer.cursor.text_col());
    assert_eq!(0, terminal.buffer.cursor.text_row());
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.process_key(Key::Backspace);
    assert_eq!(0, terminal.buffer.cursor.text_col());
    assert_eq!(0, terminal.buffer.cursor.text_row());
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.move_cursor(MoveCursor::down(1));
    assert_eq!(0, terminal.buffer.cursor.text_col());
    assert_eq!(1, terminal.buffer.cursor.text_row());
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.process_key(Key::Backspace);

    assert_eq!(1, terminal.buffer.num_lines());
    assert_eq!(0, terminal.buffer.cursor.text_row());
    assert_eq!(24, terminal.buffer.cursor.text_col());
}

#[test]
fn test_enter_at_eol() {
    let mut terminal = Window::new(10, 15);
    terminal.buffer.append_row("this is line 1.\r\n");
    terminal.buffer.append_row("this is line 2.\r\n");
    terminal.process_key(Key::End);
    terminal.process_key(Key::Return);
    assert_eq!(3, terminal.buffer.num_lines());
    assert_eq!(0, terminal.buffer.cursor.text_col());
    terminal.process_key(Key::Return);
    assert_eq!(4, terminal.buffer.num_lines());
}

#[test]
fn test_key_to_cmd() {
    use crate::commands::Cmd::*;

    let term = Window::new(1, 1);
    assert_eq!(Some(InsertChar('w')), term.key_to_cmd(Key::Other('w')));
    assert_eq!(Some(Quit), term.key_to_cmd(Key::Other(17 as char)));
    assert_eq!(
        Some(Move(MoveCursor::left(1))),
        term.key_to_cmd(Key::ArrowLeft)
    );
    assert_eq!(
        Some(Move(MoveCursor::right(1))),
        term.key_to_cmd(Key::ArrowRight)
    );
    assert_eq!(Some(Move(MoveCursor::up(1))), term.key_to_cmd(Key::ArrowUp));
    assert_eq!(
        Some(Move(MoveCursor::down(1))),
        term.key_to_cmd(Key::ArrowDown)
    );
    assert_eq!(
        Some(Move(MoveCursor::page_up(1))),
        term.key_to_cmd(Key::PageUp)
    );
    assert_eq!(
        Some(Move(MoveCursor::page_down(1))),
        term.key_to_cmd(Key::PageDown)
    );
    assert_eq!(Some(Move(MoveCursor::home())), term.key_to_cmd(Key::Home));
    assert_eq!(Some(Move(MoveCursor::end())), term.key_to_cmd(Key::End));
    assert_eq!(Some(DeleteCharForward), term.key_to_cmd(Key::Delete));
    assert_eq!(Some(DeleteCharBackward), term.key_to_cmd(Key::Backspace));
    assert_eq!(Some(Linebreak), term.key_to_cmd(Key::Return));
    assert_eq!(None, term.key_to_cmd(Key::Escape));
    assert_eq!(Some(Search), term.key_to_cmd(Key::Other(6 as char)));
    assert_eq!(
        Some(DeleteCharBackward),
        term.key_to_cmd(Key::Other(8 as char))
    );
    assert_eq!(Some(Save), term.key_to_cmd(Key::Other(19 as char)));
    for c in 0..5u8 {
        assert_eq!(None, term.key_to_cmd(Key::Other(c as char)));
    }
    assert_eq!(None, term.key_to_cmd(Key::Other(7 as char)));
    for c in 9..17u8 {
        assert_eq!(None, term.key_to_cmd(Key::Other(c as char)));
    }
    assert_eq!(None, term.key_to_cmd(Key::Other(18 as char)));
    for c in 20..32u8 {
        assert_eq!(None, term.key_to_cmd(Key::Other(c as char)));
    }
}

#[test]
fn test_empty_file() {
    let mut terminal = Window::new(10, 10);
    terminal.process_key(Key::Return);
    assert_eq!(1, terminal.buffer.num_lines());
}

#[test]
fn test_incremental_search() {
    let mut terminal = Window::new(10, 10);
    terminal
        .buffer
        .append_row("line 1. has the search text on it\r\n");
    terminal
        .buffer
        .append_row("line 2. doesn't have anything\r\n");
    terminal
        .buffer
        .append_row("line 3. also has search text here\r\n");
    terminal
        .buffer
        .append_row("line 4. another search text match\r\n");
    assert_eq!(
        Some((16, 0)),
        terminal.search_for(None, SearchDirection::Forwards, "search text")
    );
    assert_eq!(
        Some((17, 2)),
        terminal.search_for(Some((16, 0)), SearchDirection::Forwards, "search text")
    );
    assert_eq!(
        Some((16, 3)),
        terminal.search_for(Some((17, 2)), SearchDirection::Forwards, "search text")
    );
    assert_eq!(
        Some((16, 0)),
        terminal.search_for(Some((17, 2)), SearchDirection::Backwards, "search text")
    );
    assert_eq!(
        Some((17, 2)),
        terminal.search_for(Some((16, 3)), SearchDirection::Backwards, "search text")
    );
}

#[test]
fn test_newline_inside_multiline_comment() {
    let mut terminal = Window::new(100, 10);
    terminal.buffer.set_filename("test.c".to_string());
    terminal
        .buffer
        .append_row("/* this is a multiline comment */\r\n");
    terminal.buffer.append_row("int 1;\r\n");
    for _ in 0..8 {
        terminal.process_key(Key::ArrowRight);
    }
    terminal.process_key(Key::Return);
    terminal.draw_rows();
    assert!(terminal
        .append_buffer
        .contains("\x1b[36mis a multiline comment */"));
    assert!(terminal.append_buffer.contains("\x1b[32mint\x1b[39m"));
}

#[test]
fn test_backspace_inside_multiline_comment() {
    let mut terminal = Window::new(100, 10);
    terminal.buffer.set_filename("test.c".to_string());
    terminal
        .buffer
        .append_row("/* this is a multiline comment\r\n");
    terminal.buffer.append_row(" carrying on \r\n");
    terminal.buffer.append_row(" and ending */\r\n");
    terminal.process_key(Key::ArrowDown);
    terminal.process_key(Key::Backspace);
    terminal.draw_rows();
    assert!(terminal.append_buffer.contains("\x1b[36m and ending"));
}
