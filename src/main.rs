#![windows_subsystem = "console"]

use bim::config::RunConfig;
use bim::editor::Editor;
use bim::EditorImpl;
use std::env;

fn run(run_type: RunConfig) {
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

fn main() {
    let filename_arg = env::args().skip(1).nth(0);
    let run_type = match filename_arg {
        Some(arg) => {
            if arg == "--debug" {
                RunConfig::Debug
            } else {
                RunConfig::RunOpenFile(arg)
            }
        }
        _ => RunConfig::Run,
    };

    run(run_type);
}
