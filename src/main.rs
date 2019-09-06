#![windows_subsystem = "console"]

use bim::config::RunConfig;
use bim::editor::Editor;
use bim::gui::gfx_ui;
use bim::options::Options;
use bim::EditorImpl;
use std::{env, error::Error};

enum Interface {
    Terminal,
    Gui,
}

fn run_terminal(options: Options) {
    use bim::config::RunConfig::*;

    let editor = EditorImpl {};
    editor.enable_raw_mode();
    let mut terminal = editor.get_window_size();
    terminal.init();
    if let RunOpenFile(ref filename) = options.run_type {
        terminal.open(filename);
    };

    if options.run_type == Debug {
        terminal.log_debug();
    } else {
        loop {
            terminal.refresh();
            editor.process_keypress(&mut terminal);
        }
    }
}

fn run_gui(options: Options) -> Result<(), Box<dyn Error>> {
    gfx_ui::run(options)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut interface = Interface::Gui;
    let mut options = Options::default();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--debug" => options.run_type = RunConfig::Debug,
            "--no-window-system" | "-nw" => interface = Interface::Terminal,
            "--no-quit-warning" => options.no_quit_warning = true,
            _ => {
                if !arg.starts_with("-") {
                    // i.e. not a flag
                    options.run_type = RunConfig::RunOpenFile(String::from(arg))
                }
            }
        }
    }

    match interface {
        Interface::Terminal => run_terminal(options),
        Interface::Gui => run_gui(options)?,
    }

    Ok(())
}
