#![windows_subsystem = "console"]

use bim::config::RunConfig;
use bim::editor::Editor;
use bim::gui::gfx_ui;
use bim::EditorImpl;
use std::{env, error::Error};

enum Interface {
    Terminal,
    Gui,
}

fn run_terminal(run_type: RunConfig) {
    use bim::config::RunConfig::*;

    let editor = EditorImpl {};
    editor.enable_raw_mode();
    let mut terminal = editor.get_window_size();
    terminal.init();
    if let RunOpenFile(ref filename) = run_type {
        terminal.open(filename);
    };

    if run_type == Debug {
        terminal.log_debug();
    } else {
        loop {
            terminal.refresh();
            editor.process_keypress(&mut terminal);
        }
    }
}

fn run_gui(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    gfx_ui::run(run_type)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut run_type = RunConfig::Run;
    let mut interface = Interface::Gui;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--debug" => run_type = RunConfig::Debug,
            "--no-window-system" | "-nw" => interface = Interface::Terminal,
            _ => {
                if !arg.starts_with("-") {
                    // i.e. not a flag
                    run_type = RunConfig::RunOpenFile(String::from(arg))
                }
            }
        }
    }

    match interface {
        Interface::Terminal => run_terminal(run_type),
        Interface::Gui => run_gui(run_type)?,
    }

    Ok(())
}
