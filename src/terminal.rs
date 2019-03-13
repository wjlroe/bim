use crate::buffer::Buffer;
use crate::commands::{Cmd, MoveCursor, SearchDirection};
use crate::debug_log::DebugLog;
use crate::editor::BIM_VERSION;
use crate::keycodes::{ctrl_key, Key};
use crate::status::Status;
use std::error::Error;
use std::io::{stdout, Write};
use std::process::exit;
use std::time::Duration;

const UI_ROWS: i32 = 2;
const BIM_QUIT_TIMES: i8 = 3;
const BIM_DEBUG_LOG: &str = ".bim_debug";

pub struct Terminal<'a> {
    pub screen_cols: i32,
    pub screen_rows: i32,
    window_size_method: &'a str,
    pub cursor_x: i32,
    pub cursor_y: i32,
    rcursor_x: i32,
    buffer: Buffer<'a>,
    append_buffer: String,
    pub row_offset: i32,
    pub col_offset: i32,
    dirty: i32,
    quit_times: i8,
    status: Option<Status>,
    pub debug_log: DebugLog<'a>,
}

impl<'a> Terminal<'a> {
    pub fn new(screen_cols: i32, screen_rows: i32) -> Self {
        Terminal {
            screen_cols,
            screen_rows,
            window_size_method: "",
            cursor_x: 0,
            cursor_y: 0,
            rcursor_x: 0,
            buffer: Buffer::default(),
            append_buffer: String::new(),
            row_offset: 0,
            col_offset: 0,
            dirty: 0,
            quit_times: BIM_QUIT_TIMES,
            status: None,
            debug_log: DebugLog::new(BIM_DEBUG_LOG),
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
        let numrows = self.buffer.num_lines() as i32;
        for i in 0..self.screen_rows {
            let filerow = i + self.row_offset;
            if filerow >= numrows {
                if numrows == 0 && i == self.screen_rows / 3 {
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
                filerow as usize,
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
        let file_status = if self.dirty.is_positive() {
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
        let rstatus = format!(
            "{} | {}/{}",
            filetype,
            self.cursor_y + 1,
            self.buffer.num_lines()
        );
        status.truncate(self.screen_cols as usize);
        self.append_buffer.push_str(&status);
        let remaining = self.screen_cols - status.len() as i32 - rstatus.len() as i32;
        for _ in 0..remaining {
            self.append_buffer.push(' ');
        }
        self.append_buffer.push_str(&rstatus);
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
            (self.cursor_y - self.row_offset) + 1,
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
        if self.cursor_y < self.buffer.num_lines() as i32 {
            self.rcursor_x = self
                .buffer
                .text_cursor_to_render(self.cursor_x, self.cursor_y);
        }

        if self.cursor_y < self.row_offset {
            self.row_offset = self.cursor_y;
        }

        if self.cursor_y >= self.row_offset + self.screen_rows {
            self.row_offset = self.cursor_y - self.screen_rows + 1;
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

    pub fn move_cursor(&mut self, move_cursor: MoveCursor) {
        use crate::commands::Direction::*;
        use crate::commands::MoveUnit::*;

        match move_cursor {
            MoveCursor {
                unit: Rows,
                direction: Up,
                amount,
            } => {
                self.cursor_y -= amount as i32;
                // TODO: switch to unsigned, suturating_sub
                if self.cursor_y < 0 {
                    self.cursor_y = 0;
                }
            }
            MoveCursor {
                unit: Rows,
                direction: Down,
                amount,
            } => {
                self.cursor_y += amount as i32;
                if self.cursor_y > self.buffer.num_lines() as i32 {
                    self.cursor_y = self.buffer.num_lines() as i32;
                }
            }
            MoveCursor {
                unit: Rows,
                direction: Left,
                amount,
            } => {
                let mut left_amount = amount as i32;
                while left_amount > 0 {
                    if self.cursor_x != 0 {
                        self.cursor_x -= 1;
                    } else if self.cursor_y > 0 {
                        self.cursor_y -= 1;
                        self.cursor_x = self.buffer.line_len(self.cursor_y).unwrap_or(0) as i32;
                    } else {
                        break;
                    }
                    left_amount -= 1;
                }
            }
            MoveCursor {
                unit: Rows,
                direction: Right,
                amount,
            } => {
                let mut right_amount = amount as i32;
                while right_amount > 0 {
                    if let Some(row_size) = self.buffer.line_len(self.cursor_y) {
                        if self.cursor_x < row_size as i32 {
                            self.cursor_x += 1;
                        } else if self.cursor_x == row_size as i32 {
                            self.cursor_y += 1;
                            self.cursor_x = 0;
                        } else {
                            break;
                        }
                        right_amount -= 1;
                    } else {
                        break;
                    }
                }
            }
            MoveCursor {
                unit: Pages,
                direction: Down,
                amount,
            } => {
                let amount = amount * self.screen_rows as usize;
                self.move_cursor(MoveCursor::down(amount));
            }
            MoveCursor {
                unit: Pages,
                direction: Up,
                amount,
            } => {
                let amount = amount * self.screen_rows as usize;
                self.move_cursor(MoveCursor::up(amount));
            }
            MoveCursor {
                unit: Pages,
                direction: Left,
                ..
            } => {}
            MoveCursor {
                unit: Pages,
                direction: Right,
                ..
            } => {}
        }

        let rowlen = self.buffer.line_len(self.cursor_y).unwrap_or(0);

        if self.cursor_x > rowlen as i32 {
            self.cursor_x = rowlen as i32;
        }
    }

    fn insert_char(&mut self, character: char) {
        self.buffer
            .insert_char(character, self.cursor_x, self.cursor_y);
        self.cursor_x += 1;
        self.dirty += 1;
    }

    fn join_row(&mut self, at: usize) {
        if self.buffer.join_row(at) {
            self.dirty += 1;
        }
    }

    fn delete_char(&mut self) {
        let numrows = self.buffer.num_lines() as i32;
        if self.cursor_y >= numrows {
            return;
        }
        if self.cursor_x > 0 {
            self.buffer.delete_char(self.cursor_x, self.cursor_y);
            self.cursor_x -= 1;
            self.dirty += 1;
        } else if self.cursor_y > 0 && self.cursor_x == 0 {
            let at = self.cursor_y;
            self.cursor_x = self.buffer.line_len(at - 1).unwrap_or(0) as i32;
            self.join_row(at as usize);
            self.cursor_y -= 1;
        }
    }

    fn insert_newline(&mut self, row: usize, col: usize) {
        self.buffer.insert_newline(row, col);
        self.dirty += 1;
    }

    fn insert_newline_and_return(&mut self, row: usize, col: usize) {
        self.insert_newline(row, col);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    pub fn row_end(&self) -> Option<Cmd> {
        if self.cursor_y < self.buffer.num_lines() as i32 {
            Some(Cmd::JumpCursorX(
                self.buffer.line_len(self.cursor_y).unwrap_or(0),
            ))
        } else {
            None
        }
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
                if self.dirty.is_positive() && self.quit_times.is_positive() {
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
            JumpCursorX(new_x) => {
                if new_x <= self.buffer.line_len(self.cursor_y).unwrap_or(0) {
                    self.cursor_x = new_x as i32;
                }
            }
            JumpCursorY(new_y) => {
                if new_y < self.buffer.num_lines() {
                    self.cursor_y = new_y as i32;
                }
            }
            DeleteCharBackward => self.delete_char(),
            DeleteCharForward => {
                self.move_cursor(MoveCursor::right(1));
                self.delete_char();
            }
            InsertNewline(row, col) => {
                self.insert_newline(row, col);
            }
            Linebreak(row, col) => {
                self.insert_newline_and_return(row, col);
            }
            Save => self.save_file(),
            InsertChar(c) => self.insert_char(c),
            Search => {}
        }

        self.quit_times = BIM_QUIT_TIMES;
    }

    pub fn key_to_cmd(&self, key: Key) -> Option<Cmd> {
        use crate::commands::Cmd::*;
        use crate::keycodes::Key::*;

        let _ = self
            .debug_log
            .debugln_timestamped(&format!("key press: {:?}", key));
        match key {
            ArrowLeft => Some(Move(MoveCursor::left(1))),
            ArrowRight => Some(Move(MoveCursor::right(1))),
            ArrowUp => Some(Move(MoveCursor::up(1))),
            ArrowDown => Some(Move(MoveCursor::down(1))),
            PageUp => Some(Move(MoveCursor::page_up(1))),
            PageDown => Some(Move(MoveCursor::page_down(1))),
            Home => Some(JumpCursorX(0)),
            End => self.row_end(),
            Delete => Some(DeleteCharForward),
            Backspace => Some(DeleteCharBackward),
            Return => Some(Linebreak(self.cursor_y as usize, self.cursor_x as usize)),
            Escape => None,
            Control(None) => None,
            Control(Some(c)) => {
                if ctrl_key('q', c as u32) {
                    Some(Quit)
                } else if ctrl_key('f', c as u32) {
                    Some(Search)
                } else if ctrl_key('h', c as u32) {
                    Some(DeleteCharBackward)
                } else if ctrl_key('s', c as u32) {
                    Some(Save)
                } else {
                    None
                }
            }
            Other(c) => {
                let _ = self.debug_log.debugln_timestamped(&format!(
                    "other key: {character}, {key_num:x}, {key_num} as u32",
                    character = c,
                    key_num = c as u32
                ));
                if ctrl_key('h', c as u32) {
                    Some(DeleteCharBackward)
                } else if ctrl_key('q', c as u32) {
                    Some(Quit)
                } else if ctrl_key('s', c as u32) {
                    Some(Save)
                } else if ctrl_key('l', c as u32) {
                    None
                } else if ctrl_key('f', c as u32) {
                    Some(Search)
                } else if !c.is_control() {
                    Some(InsertChar(c))
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

    fn internal_save_file(&self) -> Result<usize, Box<dyn Error>> {
        self.buffer.save_to_file()
    }

    pub fn save_file(&mut self) {
        match self.internal_save_file() {
            Ok(bytes_saved) => {
                self.dirty = 0;
                self.set_status_message(format!("{} bytes written to disk", bytes_saved));
            }
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
        self.buffer
            .search_for(last_match, direction, needle)
            .and_then(|(x, y)| {
                self.cursor_x = x as i32;
                self.cursor_y = y as i32;
                self.row_offset = self.buffer.num_lines() as i32;
                Some((x, y))
            })
    }
}

#[test]
fn test_join_row() {
    let mut terminal = Terminal::new(10, 10);

    terminal.buffer.append_row("this is the first line. \r\n");
    terminal.buffer.append_row("this is the second line.\r\n");
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.join_row(1);
    assert_eq!(1, terminal.dirty);
    assert_eq!(1, terminal.buffer.num_lines());
}

#[test]
fn test_backspace_to_join_lines() {
    let mut terminal = Terminal::new(10, 10);

    terminal.buffer.append_row("this is the first line. \r\n");
    terminal.buffer.append_row("this is second line.\r\n");
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.process_key(Key::Backspace);
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.move_cursor(MoveCursor::down(1));
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(1, terminal.cursor_y);
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.process_key(Key::Backspace);

    assert_eq!(1, terminal.buffer.num_lines());
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(24, terminal.cursor_x);
}

#[test]
fn test_insert_newline() {
    let mut terminal = Terminal::new(10, 15);
    terminal.buffer.append_row("what a good first line.\r\n");
    terminal.buffer.append_row("not a bad second line\r\n");
    assert_eq!(2, terminal.buffer.num_lines());

    terminal.insert_newline(1, 0);

    assert_eq!(3, terminal.buffer.num_lines());
    assert_eq!(1, terminal.dirty);

    terminal.insert_newline(2, 4);

    assert_eq!(4, terminal.buffer.num_lines());
    assert_eq!(2, terminal.dirty);
}

#[test]
fn test_enter_at_eol() {
    let mut terminal = Terminal::new(10, 15);
    terminal.buffer.append_row("this is line 1.\r\n");
    terminal.buffer.append_row("this is line 2.\r\n");
    terminal.process_key(Key::End);
    terminal.process_key(Key::Return);
    assert_eq!(3, terminal.buffer.num_lines());
    assert_eq!(0, terminal.cursor_x);
    terminal.process_key(Key::Return);
    assert_eq!(4, terminal.buffer.num_lines());
}

#[test]
fn test_key_to_cmd() {
    use crate::commands::Cmd::*;

    let term = Terminal::new(1, 1);
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
    assert_eq!(Some(JumpCursorX(0)), term.key_to_cmd(Key::Home));
    assert_eq!(Some(DeleteCharForward), term.key_to_cmd(Key::Delete));
    assert_eq!(Some(DeleteCharBackward), term.key_to_cmd(Key::Backspace));
    assert_eq!(Some(Linebreak(0, 0)), term.key_to_cmd(Key::Return));
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
fn test_jump_to_end() {
    use crate::commands::Cmd::*;

    let mut term = Terminal::new(10, 10);
    assert_eq!(None, term.key_to_cmd(Key::End));
    term.buffer.append_row("this is a line of text.\r\n");
    assert_eq!(Some(JumpCursorX(23)), term.key_to_cmd(Key::End));
}

#[test]
fn test_empty_file() {
    let mut terminal = Terminal::new(10, 10);
    terminal.process_key(Key::Return);
    assert_eq!(1, terminal.buffer.num_lines());
}

#[test]
fn test_incremental_search() {
    let mut terminal = Terminal::new(10, 10);
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
    let mut terminal = Terminal::new(100, 10);
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
    let mut terminal = Terminal::new(100, 10);
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
