use keycodes::{Key, ctrl_key};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, stdout};
use std::process::exit;

const BIM_VERSION: &str = "0.0.1";

pub struct Terminal {
    pub cols: i32,
    pub rows: i32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub append_buffer: String,
    pub row: String,
    pub numrows: i32,
}

impl Terminal {
    pub fn new(cols: i32, rows: i32) -> Self {
        Terminal {
            cols,
            rows,
            cursor_x: 0,
            cursor_y: 0,
            append_buffer: String::new(),
            row: String::new(),
            numrows: 0,
        }
    }

    fn die(&mut self, message: &str) {
        self.reset();

        println!("Error: {}", message);
        exit(1);
    }

    fn draw_rows(&mut self) {
        for i in 0..self.rows {
            if i >= self.numrows {
                if self.numrows == 0 && i == self.rows / 3 {
                    let mut welcome = format!("bim editor - version {}",
                                              BIM_VERSION);
                    welcome.truncate(self.cols as usize);
                    let mut padding = (self.cols - welcome.len() as i32) / 2;
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
                let mut row = self.row.clone();
                row.truncate(self.cols as usize);
                self.append_buffer.push_str(&row);
            }

            self.clear_line();

            if i < self.rows - 1 {
                self.append_buffer.push_str("\r\n");
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
        let ansi = format!("\x1b[{};{}H", self.cursor_y + 1, self.cursor_x + 1);
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

    pub fn refresh(&mut self) {
        self.hide_cursor();
        self.goto_origin();

        self.draw_rows();

        self.reset_cursor();

        self.show_cursor();

        self.flush();
    }

    pub fn move_cursor(&mut self, key: Key) {
        match key {
            Key::ArrowUp => {
                if self.cursor_y != 0 {
                    self.cursor_y -= 1;
                }
            }
            Key::ArrowDown => {
                if self.cursor_y != self.rows - 1 {
                    self.cursor_y += 1;
                }
            }
            Key::ArrowLeft => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                }
            }
            Key::ArrowRight => {
                if self.cursor_x != self.cols - 1 {
                    self.cursor_x += 1;
                }
            }
            _ => {}
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
                for _ in 0..self.rows {
                    self.move_cursor(up_or_down);
                }
            }
            Home => {
                self.cursor_x = 0;
            }
            End => {
                self.cursor_x = self.cols - 1;
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

    fn open(&mut self, filename: String) {
        match File::open(filename) {
            Ok(f) => {
                let mut reader = BufReader::new(f);
                let mut buffer = String::new();
                let _ = reader.read_line(&mut buffer);
                self.numrows = 1;
                self.row.clear();
                self.row
                    .push_str(buffer.trim_right_matches(|c| {
                                                            c == '\r' ||
                                                            c == '\n'
                                                        }));
            }
            Err(e) => self.die(e.description()),
        }
    }

    pub fn init(&mut self, filename_arg: Option<String>) {
        if let Some(filename) = filename_arg {
            self.open(filename);
        }
    }
}
