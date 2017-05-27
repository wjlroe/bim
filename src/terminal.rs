use keycodes::{Key, ctrl_key};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, stdout};
use std::process::exit;
use std::time::{Duration, Instant};

const BIM_VERSION: &str = "0.0.1";
const TAB_STOP: usize = 8;

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
        row.chars.push_str(text);
        row.size = text.len();
        let mut rsize = 0;
        for source_char in text.chars() {
            if source_char == '\t' {
                row.render.push(' ');
                rsize += 1;
                while rsize % TAB_STOP != 0 {
                    row.render.push(' ');
                    rsize += 1;
                }
            } else {
                row.render.push(source_char);
                rsize += 1;
            }
        }
        row.rsize = rsize;
        row
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
}

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
                    let mut welcome = format!("bim editor - version {}",
                                              BIM_VERSION);
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
        let filename = self.filename
            .clone()
            .unwrap_or(String::from("[No Name]"));
        let mut status =
            format!("{0:.20} - {1} lines", filename, self.rows.len());
        let rstatus = format!("{}/{}", self.cursor_y + 1, self.rows.len());
        status.truncate(self.screen_cols as usize);
        self.append_buffer.push_str(&status);
        let remaining = self.screen_cols - status.len() as i32 -
                        rstatus.len() as i32;
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
        let ansi = format!("\x1b[{};{}H",
                           (self.cursor_y - self.row_offset) + 1,
                           (self.rcursor_x - self.col_offset) + 1);
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
            Key::ArrowUp => {
                if self.cursor_y != 0 {
                    self.cursor_y -= 1;
                }
            }
            Key::ArrowDown => {
                if self.cursor_y < self.rows.len() as i32 {
                    self.cursor_y += 1;
                }
            }
            Key::ArrowLeft => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.rows[self.cursor_y as usize].size as
                                    i32;
                }
            }
            Key::ArrowRight => {
                if let Some(row) = current_row {
                    if self.cursor_x < row.size as i32 {
                        self.cursor_x += 1;
                    } else if self.cursor_x == row.size as i32 {
                        self.cursor_y += 1;
                        self.cursor_x = 0;
                    }
                }
            }
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
            End => {
                if self.cursor_y < self.rows.len() as i32 {
                    self.cursor_x = self.rows[self.cursor_y as usize].size as
                                    i32;
                }
            }
            Delete => {}
            Other(c) => {
                if ctrl_key('q', c as u32) {
                    self.reset();
                    exit(0);
                }
            }
        }
    }

    fn set_status_message(&mut self, message: String) {
        let status = Status::new(message);
        self.status = Some(status);
    }

    fn open(&mut self, filename: String) {
        match File::open(&filename) {
            Ok(f) => {
                self.filename = Some(filename);
                self.rows.clear();
                for line in BufReader::new(f).lines() {
                    let row = Row::new(&line.unwrap());
                    self.rows.push(row);
                }
            }
            Err(e) => self.die(e.description()),
        }
    }

    pub fn init(&mut self, filename_arg: Option<String>) {
        if let Some(filename) = filename_arg {
            self.open(filename);
        }

        self.set_status_message(String::from("HELP: Ctrl-Q = quit"));

        self.screen_rows -= 2;
    }
}
