use keycodes::{ctrl_key, Key};
use std::cmp::Ordering;
use std::default::Default;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, BufWriter, Write};
use std::process::exit;
use std::time::{Duration, Instant};

const BIM_VERSION: &str = "0.0.1";
const TAB_STOP: usize = 8;
const UI_ROWS: i32 = 2;
const BIM_QUIT_TIMES: i8 = 3;

#[derive(Default, PartialEq, Eq)]
struct Row {
    chars: String,
    size: usize,
    render: String,
    rsize: usize,
}

impl Row {
    fn new(text: &str) -> Self {
        let mut row = Row {
            chars: String::new(),
            size: 0,
            render: String::new(),
            rsize: 0,
        };
        row.set_text(text);
        row
    }

    fn set_text(&mut self, text: &str) {
        self.chars.clear();
        self.chars.push_str(text);
        self.update();
    }

    fn update(&mut self) {
        let mut string_end = self.chars.len();
        while string_end > 0 &&
            (self.chars.chars().nth(string_end - 1).unwrap() == '\n' ||
                self.chars.chars().nth(string_end - 1).unwrap() == '\r')
        {
            string_end -= 1;
        }
        self.size = string_end;
        self.update_render();
    }

    fn update_render(&mut self) {
        self.render.clear();
        let mut rsize = 0;
        for source_char in self.chars.chars() {
            if source_char == '\t' {
                self.render.push(' ');
                rsize += 1;
                while rsize % TAB_STOP != 0 {
                    self.render.push(' ');
                    rsize += 1;
                }
            } else if source_char == '\n' || source_char == '\r' {
                continue;
            } else {
                self.render.push(source_char);
                rsize += 1;
            }
        }
        self.rsize = rsize;
    }

    fn text_cursor_to_render(&self, cidx: i32) -> i32 {
        let tab_stop = TAB_STOP as i32;
        let mut ridx: i32 = 0;
        for (i, source_char) in self.chars.chars().enumerate() {
            if i == cidx as usize {
                break;
            }
            if source_char == '\t' {
                ridx += (tab_stop - 1) - (ridx % tab_stop);
            }
            ridx += 1;
        }
        ridx
    }

    fn insert_char(&mut self, at: usize, character: char) {
        let at = if at > self.size { self.size } else { at };
        self.chars.insert(at, character);
        self.size += 1;
        self.update_render();
    }

    fn delete_char(&mut self, at: usize) {
        let at = if at >= self.size { self.size - 1 } else { at };
        self.chars.remove(at);
        self.update();
    }
}

#[test]
fn test_row_insert_char() {
    let mut row = Row::new("a line of text\r\n");
    assert_eq!(14, row.size);
    assert_eq!(14, row.rsize);
    assert_eq!(row.chars.trim(), row.render);
    row.insert_char(2, 'z');
    assert_eq!(15, row.size);
    assert_eq!(15, row.rsize);
    assert_eq!("a zline of text\r\n", row.chars);
    row.insert_char(0, '_');
    assert_eq!(16, row.size);
    assert_eq!(16, row.rsize);
    assert_eq!("_a zline of text\r\n", row.chars);
    row.insert_char(16, '_');
    assert_eq!(17, row.size);
    assert_eq!(17, row.rsize);
    assert_eq!("_a zline of text_\r\n", row.chars);
}

#[test]
fn test_row_update() {
    let mut row = Row::default();
    row.update();

    assert_eq!(0, row.size);
    assert_eq!(0, row.rsize);

    row.set_text("a row\n");
    row.update();

    assert_eq!(5, row.size);
    assert_eq!(5, row.rsize);

    row.set_text("another row\r\n");
    row.update();

    assert_eq!(11, row.size);
    assert_eq!(11, row.rsize);
}

#[test]
fn test_row_delete_char() {
    let mut row = Row::new("this is a nice row\r\n");
    assert_eq!(18, row.size);
    assert_eq!("this is a nice row", row.render);

    row.delete_char(0);
    assert_eq!("his is a nice row\r\n", row.chars);
    assert_eq!(17, row.size);
    assert_eq!("his is a nice row", row.render);

    row.delete_char(17);
    assert_eq!("his is a nice ro\r\n", row.chars);
    assert_eq!(16, row.size);
    assert_eq!("his is a nice ro", row.render);
}

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
    filename: Option<String>,
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
                let onscreen_row = self.rows[filerow as usize]
                    .render
                    .chars()
                    .skip(self.col_offset as usize)
                    .take(self.screen_cols as usize)
                    .collect::<String>();
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

    pub fn move_cursor(&mut self, key: Key) {
        let current_row = self.rows.get(self.cursor_y as usize);

        match key {
            Key::ArrowUp => if self.cursor_y != 0 {
                self.cursor_y -= 1;
            },
            Key::ArrowDown => if self.cursor_y < self.rows.len() as i32 {
                self.cursor_y += 1;
            },
            Key::ArrowLeft => if self.cursor_x != 0 {
                self.cursor_x -= 1;
            } else if self.cursor_y > 0 {
                self.cursor_y -= 1;
                self.cursor_x = self.rows[self.cursor_y as usize].size as i32;
            },
            Key::ArrowRight => if let Some(row) = current_row {
                if self.cursor_x < row.size as i32 {
                    self.cursor_x += 1;
                } else if self.cursor_x == row.size as i32 {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
            },
            _ => {}
        }

        let rowlen = match self.rows.get(self.cursor_y as usize) {
            Some(row) => row.size,
            _ => 0,
        };

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

    fn delete_char(&mut self) {
        let numrows = self.rows.len() as i32;
        if self.cursor_y < numrows && self.cursor_x > 0 {
            self.rows[self.cursor_y as usize]
                .delete_char((self.cursor_x - 1) as usize);
            self.cursor_x -= 1;
            self.dirty += 1;
        }
    }

    pub fn process_key(&mut self, key: Key) {
        use keycodes::Key::*;

        match key {
            ArrowLeft | ArrowRight | ArrowUp | ArrowDown => {
                self.move_cursor(key)
            }
            PageUp | PageDown => {
                let up_or_down =
                    if key == PageUp { ArrowUp } else { ArrowDown };

                if up_or_down == ArrowUp {
                    self.cursor_y = self.row_offset;
                } else {
                    self.cursor_y = self.row_offset + self.screen_rows - 1;
                    if self.cursor_y > self.rows.len() as i32 {
                        self.cursor_y = self.rows.len() as i32;
                    }
                }

                for _ in 0..self.screen_rows {
                    self.move_cursor(up_or_down);
                }
            }
            Home => {
                self.cursor_x = 0;
            }
            End => if self.cursor_y < self.rows.len() as i32 {
                self.cursor_x = self.rows[self.cursor_y as usize].size as i32;
            },
            Delete | Backspace => {
                if key == Delete {
                    self.move_cursor(ArrowRight);
                }
                self.delete_char();
            }
            Return | Escape => {}
            Other(c) => if ctrl_key('q', c as u32) {
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
            } else if ctrl_key('h', c as u32) {
                self.delete_char();
            } else if ctrl_key('l', c as u32) {
            } else if ctrl_key('s', c as u32) {
                self.save_file();
            } else {
                self.insert_char(c);
            },
        }
        self.quit_times = BIM_QUIT_TIMES;
    }

    fn set_status_message(&mut self, message: String) {
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
                    let mut row = Row::default();
                    let line = reader.read_line(&mut row.chars);
                    match line {
                        Ok(bytes_read) if bytes_read > 0 => {
                            row.update();
                            self.rows.push(row);
                        }
                        _ => break,
                    }
                }
            }
            Err(e) => self.die(e.description()),
        }
    }

    pub fn init(&mut self) {
        self.set_status_message(
            String::from("HELP: Ctrl-S = save | Ctrl-Q = quit"),
        );

        self.screen_rows -= UI_ROWS;
    }

    pub fn log_debug(&self) -> Result<(), Box<Error>> {
        let mut buffer = File::create(".kilo_debug")?;
        buffer.write(&format!("rows: {}\r\n", self.screen_rows + UI_ROWS)
            .into_bytes())?;
        buffer.write(&format!("cols: {}\r\n", self.screen_cols).into_bytes())?;
        buffer.flush()?;
        Ok(())
    }

    fn internal_save_file(&self) -> Result<usize, Box<Error>> {
        let mut bytes_saved: usize = 0;
        if let Some(ref filename) = self.filename {
            let mut buffer = BufWriter::new(File::create(filename)?);
            for line in &self.rows {
                bytes_saved += buffer.write(line.chars.as_bytes())?;
            }
            buffer.flush()?;
        }
        Ok(bytes_saved)
    }

    pub fn save_file(&mut self) {
        match self.internal_save_file() {
            Ok(bytes_saved) => {
                self.dirty = 0;
                self.set_status_message(
                    format!("{} bytes written to disk", bytes_saved),
                );
            }
            Err(err) => {
                self.set_status_message(format!("Can't save! Error: {:?}", err))
            }
        }
    }
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
