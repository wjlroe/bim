use std::io::{Write, stdout};

pub struct Terminal {
    pub cols: i32,
    pub rows: i32,
}

impl Terminal {
    pub fn new(cols: i32, rows: i32) -> Self {
        Terminal { cols, rows }
    }
}

pub fn clear_screen() {
    print!("\x1b[2J");
    print!("\x1b[H");
    stdout().flush().unwrap();
}

pub fn refresh_screen(terminal: &Terminal) {
    clear_screen();

    draw_rows(terminal);

    print!("\x1b[H");

    stdout().flush().unwrap();
}

fn draw_rows(terminal: &Terminal) {
    for _ in 1..terminal.rows {
        print!("~\r\n");
    }
}
