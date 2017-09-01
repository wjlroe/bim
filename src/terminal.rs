use commands::{Cmd, MoveCursor};
use keycodes::{ctrl_key, Key};
use row::Row;
use std::cmp::Ordering;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{stdout, BufRead, BufReader, BufWriter, Write};
use std::process::exit;
use std::time::{Duration, Instant};
use time::now;

const BIM_VERSION: &str = "0.0.1";
const UI_ROWS: i32 = 2;
const BIM_QUIT_TIMES: i8 = 3;
const BIM_DEBUG_LOG: &str = ".kilo_debug";

#[derive(PartialEq, Eq)]
struct Status {
    message: String,
    time: Instant,
}

impl Status {
    fn new(message: String) -> Self {
        Status {
            message,
            time: Instant::now(),
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct Terminal {
    pub screen_cols: i32,
    pub screen_rows: i32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    rcursor_x: i32,
    pub append_buffer: String,
    rows: Vec<Row>,
    row_offset: i32,
    col_offset: i32,
    pub filename: Option<String>,
    dirty: i32,
    quit_times: i8,
    status: Option<Status>,
}

impl Terminal {
    pub fn new(screen_cols: i32, screen_rows: i32) -> Self {
        Terminal {
            screen_cols,
            screen_rows,
            cursor_x: 0,
            cursor_y: 0,
            rcursor_x: 0,
            append_buffer: String::new(),
            rows: Vec::new(),
            row_offset: 0,
            col_offset: 0,
            filename: None,
            dirty: 0,
            quit_times: BIM_QUIT_TIMES,
            status: None,
        }
    }

    fn die(&mut self, message: &str) {
        self.reset();

        println!("Error: {}", message);
        exit(1);
    }

    fn draw_rows(&mut self) {
        let numrows = self.rows.len() as i32;
        for i in 0..self.screen_rows {
            let filerow = i + self.row_offset;
            if filerow >= numrows {
                if numrows == 0 && i == self.screen_rows / 3 {
                    let mut welcome =
                        format!("bim editor - version {}", BIM_VERSION);
                    welcome.truncate(self.screen_cols as usize);
                    let mut padding =
                        (self.screen_cols - welcome.len() as i32) / 2;
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
            } else {
                let onscreen_row = self.rows[filerow as usize].onscreen_text(
                    self.col_offset as usize,
                    self.screen_cols as usize,
                );
                self.append_buffer.push_str(onscreen_row.as_str());
            }

            self.clear_line();

            self.append_buffer.push_str("\r\n");
        }
    }

    fn draw_status_bar(&mut self) {
        self.append_buffer.push_str("\x1b[7m");
        let filename =
            self.filename.clone().unwrap_or(String::from("[No Name]"));
        let file_status = if self.dirty.is_positive() {
            "(modified)"
        } else {
            ""
        };
        let mut status = format!(
            "{0:.20} - {1} lines {2}",
            filename,
            self.rows.len(),
            file_status
        );
        let rstatus = format!("{}/{}", self.cursor_y + 1, self.rows.len());
        status.truncate(self.screen_cols as usize);
        self.append_buffer.push_str(&status);
        let remaining =
            self.screen_cols - status.len() as i32 - rstatus.len() as i32;
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
            if status.time.elapsed() < Duration::from_secs(5) {
                let mut msg = status.message.clone();
                msg.truncate(self.screen_cols as usize);
                self.append_buffer.push_str(&msg);
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
        self.flush();
    }

    fn flush(&mut self) {
        {
            let output = self.append_buffer.as_bytes();
            if stdout().write(output).unwrap() == output.len() {
                stdout().flush().unwrap();
            } else {
                panic!("oh no, couldn't write append buffer to screen!");
            }
        }
        self.append_buffer.clear();
    }

    fn scroll(&mut self) {
        self.rcursor_x = 0;
        if self.cursor_y < self.rows.len() as i32 {
            self.rcursor_x = self.rows[self.cursor_y as usize]
                .text_cursor_to_render(self.cursor_x);
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

        self.flush();
    }

    pub fn move_cursor(&mut self, move_cursor: MoveCursor) {
        use commands::Direction::*;
        use commands::MoveUnit::*;

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
                if self.cursor_y > self.rows.len() as i32 {
                    self.cursor_y = self.rows.len() as i32;
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
                        self.cursor_x =
                            self.rows[self.cursor_y as usize].size as i32;
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
                    if let Some(row) = self.rows.get(self.cursor_y as usize) {
                        if self.cursor_x < row.size as i32 {
                            self.cursor_x += 1;
                        } else if self.cursor_x == row.size as i32 {
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

        let rowlen = self.rows
            .get(self.cursor_y as usize)
            .map(|r| r.size)
            .unwrap_or(0);

        if self.cursor_x > rowlen as i32 {
            self.cursor_x = rowlen as i32;
        }
    }

    fn insert_char(&mut self, character: char) {
        if self.cursor_y == self.rows.len() as i32 {
            self.rows.push(Row::new(""));
        }
        self.rows[self.cursor_y as usize]
            .insert_char(self.cursor_x as usize, character);
        self.cursor_x += 1;
        self.dirty += 1;
    }

    fn join_row(&mut self, at: usize) {
        if at > 0 && at < self.rows.len() {
            let row = self.rows.remove(at);
            if let Some(previous_row) = self.rows.get_mut(at - 1) {
                previous_row.append_text(row.as_str());
            }
            self.dirty += 1;
        }
    }

    fn delete_char(&mut self) {
        let numrows = self.rows.len() as i32;
        if self.cursor_y >= numrows {
            return;
        }
        if self.cursor_x > 0 {
            self.rows[self.cursor_y as usize]
                .delete_char((self.cursor_x - 1) as usize);
            self.cursor_x -= 1;
            self.dirty += 1;
        } else if self.cursor_y > 0 && self.cursor_x == 0 {
            let at = self.cursor_y as usize;
            self.cursor_x = self.rows[at - 1].size as i32;
            self.join_row(at);
            self.cursor_y -= 1;
        }
    }

    fn insert_row(&mut self, at: usize, text: &str) {
        if at <= self.rows.len() {
            let row = Row::new(text);
            self.rows.insert(at, row);
        }
    }

    fn append_row(&mut self, text: &str) {
        let at = self.rows.len();
        self.insert_row(at, text);
    }

    fn insert_newline(&mut self, row: usize, col: usize) {
        let newline = self.rows[row].newline();
        if col == 0 {
            self.insert_row(row, &newline);
        } else {
            let new_line_text = self.rows[row].truncate(col);
            self.insert_row(row + 1, &new_line_text);
        }
        self.dirty += 1;
    }

    fn insert_newline_and_return(&mut self, row: usize, col: usize) {
        self.insert_newline(row, col);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    pub fn row_end(&self) -> Option<Cmd> {
        if self.cursor_y < self.rows.len() as i32 {
            Some(Cmd::JumpCursorX(self.rows[self.cursor_y as usize].size))
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
        use commands::Cmd::*;

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
                if new_x <= self.rows[self.cursor_y as usize].size {
                    self.cursor_x = new_x as i32;
                }
            }
            JumpCursorY(new_y) => if new_y < self.rows.len() {
                self.cursor_y = new_y as i32;
            },
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
        }

        self.quit_times = BIM_QUIT_TIMES;
    }

    pub fn key_to_cmd(&self, key: Key) -> Option<Cmd> {
        use keycodes::Key::*;
        use commands::Cmd::*;

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
            Return => {
                Some(Linebreak(self.cursor_y as usize, self.cursor_x as usize))
            }
            Escape => None,
            Other(c) => {
                self.debug(format!("other key: {}, {} as u32\n", c, c as u32));
                if ctrl_key('h', c as u32) {
                    Some(DeleteCharBackward)
                } else if ctrl_key('q', c as u32) {
                    Some(Quit)
                } else if ctrl_key('s', c as u32) {
                    Some(Save)
                } else if ctrl_key('l', c as u32) {
                    None
                } else if !c.is_control() {
                    Some(InsertChar(c))
                } else {
                    None
                }
            }
        }
    }

    pub fn set_status_message(&mut self, message: String) {
        let status = Status::new(message);
        self.status = Some(status);
    }

    pub fn open(&mut self, filename: &str) {
        match File::open(filename) {
            Ok(f) => {
                self.filename = Some(filename.to_string());
                self.rows.clear();
                let mut reader = BufReader::new(f);
                loop {
                    let mut line = String::new();
                    let read_info = reader.read_line(&mut line);
                    match read_info {
                        Ok(bytes_read) if bytes_read > 0 => {
                            self.append_row(&line);
                        }
                        _ => break,
                    }
                }
            }
            Err(e) => self.die(e.description()),
        }
    }

    pub fn init(&mut self) {
        self.start_debug();
        self.set_status_message(
            String::from("HELP: Ctrl-S = save | Ctrl-Q = quit"),
        );

        self.screen_rows -= UI_ROWS;
    }

    fn start_debug(&self) {
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(BIM_DEBUG_LOG)
        {
            let _ =
                file.write(&format!("bim version {} starting\n", BIM_VERSION)
                    .into_bytes());
            let _ = file.flush();
        }
    }

    fn debug(&self, text: String) {
        if let Ok(mut file) =
            OpenOptions::new().append(true).open(BIM_DEBUG_LOG)
        {
            let now = now();
            let _ = file.write(&format!("{}: ", now.rfc822()).into_bytes());
            let _ = file.write(&text.into_bytes());
            let _ = file.flush();
        }
    }

    pub fn log_debug(&self) {
        self.debug(format!("rows: {}\r\n", self.screen_rows + UI_ROWS));
        self.debug(format!("cols: {}\r\n", self.screen_cols));
    }

    fn internal_save_file(&self) -> Result<usize, Box<Error>> {
        let mut bytes_saved: usize = 0;
        if let Some(ref filename) = self.filename {
            let mut buffer = BufWriter::new(File::create(filename)?);
            for line in &self.rows {
                bytes_saved += buffer.write(line.as_str().as_bytes())?;
            }
            buffer.flush()?;
        }
        Ok(bytes_saved)
    }

    pub fn save_file(&mut self) {
        if self.filename.is_some() {
            match self.internal_save_file() {
                Ok(bytes_saved) => {
                    self.dirty = 0;
                    self.set_status_message(
                        format!("{} bytes written to disk", bytes_saved),
                    );
                }
                Err(err) => {
                    self.set_status_message(
                        format!("Can't save! Error: {:?}", err),
                    )
                }
            }
        }
    }
}

#[test]
fn test_join_row() {
    let mut terminal = Terminal::new(10, 10);

    terminal.append_row("this is the first line. \r\n");
    terminal.append_row("this is the second line.\r\n");
    assert_eq!(2, terminal.rows.len());

    terminal.join_row(1);
    assert_eq!(1, terminal.dirty);
    assert_eq!(1, terminal.rows.len());
    let first_row = terminal.rows.get(0).clone().unwrap();
    assert_eq!(
        "this is the first line. this is the second line.\r\n",
        first_row.as_str()
    );
}

#[test]
fn test_backspace_to_join_lines() {
    let mut terminal = Terminal::new(10, 10);

    terminal.append_row("this is the first line. \r\n");
    terminal.append_row("this is second line.\r\n");
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(2, terminal.rows.len());

    terminal.process_key(Key::Backspace);
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(2, terminal.rows.len());

    terminal.move_cursor(MoveCursor::down(1));
    assert_eq!(0, terminal.cursor_x);
    assert_eq!(1, terminal.cursor_y);
    assert_eq!(2, terminal.rows.len());

    terminal.process_key(Key::Backspace);

    assert_eq!(1, terminal.rows.len());
    assert_eq!(0, terminal.cursor_y);
    assert_eq!(24, terminal.cursor_x);
}

#[test]
fn test_insert_newline() {
    let mut terminal = Terminal::new(10, 15);
    terminal.append_row("what a good first line.\r\n");
    terminal.append_row("not a bad second line\r\n");
    assert_eq!(2, terminal.rows.len());

    terminal.insert_newline(1, 0);

    assert_eq!(3, terminal.rows.len());
    assert_eq!(1, terminal.dirty);
    assert_eq!(
        vec![
            "what a good first line.\r\n",
            "\r\n",
            "not a bad second line\r\n",
        ],
        terminal
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );

    terminal.insert_newline(2, 4);

    assert_eq!(4, terminal.rows.len());
    assert_eq!(2, terminal.dirty);
    assert_eq!(
        vec![
            "what a good first line.\r\n",
            "\r\n",
            "not \r\n",
            "a bad second line\r\n",
        ],
        terminal
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["what a good first line.", "", "not ", "a bad second line"],
        terminal
            .rows
            .iter()
            .map(|r| r.rendered_str().clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_enter_at_eol() {
    let mut terminal = Terminal::new(10, 15);
    terminal.append_row("this is line 1.\r\n");
    terminal.append_row("this is line 2.\r\n");
    terminal.process_key(Key::End);
    terminal.process_key(Key::Return);
    assert_eq!(3, terminal.rows.len());
    assert_eq!(0, terminal.cursor_x);
    terminal.process_key(Key::Return);
    assert_eq!(4, terminal.rows.len());
}

impl Ord for Terminal {
    fn cmp(&self, other: &Terminal) -> Ordering {
        self.screen_rows
            .cmp(&other.screen_rows)
            .then(self.screen_cols.cmp(&other.screen_cols))
    }
}

impl PartialOrd for Terminal {
    fn partial_cmp(&self, other: &Terminal) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[test]
fn test_terminal_ordering() {
    use std::cmp::Ordering::*;

    let term1 = Terminal::new(1, 1);
    assert_eq!(Equal, term1.cmp(&term1));
    let term2 = Terminal::new(2, 1);
    assert_eq!(Less, term1.cmp(&term2));
    assert_eq!(Greater, term2.cmp(&term1));
    let term3 = Terminal::new(1, 2);
    assert_eq!(Less, term1.cmp(&term3));
    assert_eq!(Greater, term3.cmp(&term1));

    let none_term: Option<Terminal> = None;
    let some_term1 = Some(term1);
    assert_eq!(Equal, some_term1.cmp(&some_term1));
    assert_eq!(Less, none_term.cmp(&some_term1));
    assert_eq!(Greater, some_term1.cmp(&none_term));
}

#[test]
fn test_key_to_cmd() {
    use commands::Cmd::*;

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
    assert_eq!(
        Some(DeleteCharBackward),
        term.key_to_cmd(Key::Other(8 as char))
    );
    assert_eq!(Some(Save), term.key_to_cmd(Key::Other(19 as char)));
    for c in 0..8u8 {
        assert_eq!(None, term.key_to_cmd(Key::Other(c as char)));
    }
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
    use commands::Cmd::*;

    let mut term = Terminal::new(10, 10);
    assert_eq!(None, term.key_to_cmd(Key::End));
    term.append_row("this is a line of text.\r\n");
    assert_eq!(Some(JumpCursorX(23)), term.key_to_cmd(Key::End));
}

#[test]
fn test_insert_char() {
    let mut terminal = Terminal::new(10, 10);
    terminal.insert_char('£');
    terminal.insert_char('1');
    assert_eq!(
        vec!["£1"],
        terminal
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );
}
