use std::io::{Write, stdout};

pub fn clear_screen() {
    print!("\x1b[2J");
    print!("\x1b[H");
    stdout().flush().unwrap();
}

pub fn refresh_screen() {
    clear_screen();

    draw_rows();

    print!("\x1b[H");

    stdout().flush().unwrap();
}

fn draw_rows() {
    for _ in 0..24 {
        print!("~\r\n");
    }
}
