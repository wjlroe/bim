use std::io::{Write, stdout};

pub fn refresh_screen() {
    print!("\x1b[2J");
    print!("\x1b[H");
    stdout().flush().unwrap();
}
