use std::io::{Write, stdout};

pub struct Terminal {
    pub cols: i32,
    pub rows: i32,
    pub append_buffer: String,
}

impl Terminal {
    pub fn new(cols: i32, rows: i32) -> Self {
        Terminal {
            cols,
            rows,
            append_buffer: String::new(),
        }
    }

    fn draw_rows(&mut self) {
        for i in 1..self.rows {
            self.append_buffer.push_str("~");
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

        self.goto_origin();
        self.show_cursor();

        self.flush();
    }
}
