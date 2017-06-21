#![windows_subsystem = "console"]
extern crate kilo;

use std::env;

fn main() {
    let filename_arg = env::args().skip(1).nth(0);

    kilo::run(filename_arg);
}
