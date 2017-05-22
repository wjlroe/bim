use keycodes::{Key, ctrl_key};
use std::io::{Write, stdout};
use std::process::exit;

const BIM_VERSION: &str = "0.0.1";

pub struct Terminal {
    pub cols: i32,
    pub rows: i32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub append_buffer: String,
}

impl Terminal {
    pub fn new(cols: i32, rows: i32) -> Self {
        Terminal {
            cols,
            rows,
            cursor_x: 0,
            cursor_y: 0,
            append_buffer: String::new(),
        }
    }

    fn draw_rows(&mut self) {
        for i in 0..self.rows {
            if i == self.rows / 3 {
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
            Other(c) => {
                if ctrl_key('q', c as u32) {
                    self.reset();
                    exit(0);
                }
            }
        }
    }
}
