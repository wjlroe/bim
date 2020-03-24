#![windows_subsystem = "windows"]

use bim::config::RunConfig;
use bim::gui::gfx_ui;
use bim::options::Options;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = Options::default();
    let mut files = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--debug" => options.run_type = RunConfig::Debug,
            "--no-quit-warning" => options.no_quit_warning = true,
            "-O" => options.vsplit = true,
            _ => {
                if !arg.starts_with("-") {
                    // i.e. not a flag
                    files.push(String::from(arg));
                }
            }
        }
    }

    if files.len() > 0 {
        options.run_type = RunConfig::RunOpenFiles(files);
    }

    gfx_ui::run(options)?;

    Ok(())
}
