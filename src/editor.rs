use crate::commands::{Cmd, SearchDirection};
use crate::keycodes::{ctrl_key, Key};
use crate::terminal::Terminal;

pub const BIM_VERSION: &str = "0.0.1";

#[cfg(windows)]
pub const DEFAULT_NEWLINE: &str = "\r\n";
#[cfg(not(windows))]
pub const DEFAULT_NEWLINE: &str = "\n";

pub trait Editor {
    fn enable_raw_mode(&self);
    fn get_window_size(&self) -> Terminal<'_>;
    fn read_a_character(&self) -> Option<Key>;

    fn prompt<F>(
        &self,
        terminal: &mut Terminal<'_>,
        status_left: &str,
        status_right: &str,
        mut callback: F,
    ) -> Option<String>
    where
        F: FnMut(&mut Terminal<'_>, &str, Key),
    {
        let mut entered_text = String::new();
        loop {
            terminal.set_status_message(format!(
                "{} {} {}",
                status_left, entered_text, status_right
            ));
            terminal.refresh();
            if let Some(key) = self.read_a_character() {
                match key {
                    Key::Other(c) if !c.is_control() => {
                        entered_text.push(c);
                    }
                    Key::Return if !entered_text.is_empty() => {
                        terminal.set_status_message(String::new());
                        callback(terminal, &entered_text, key);
                        break;
                    }
                    Key::Escape => {
                        terminal.set_status_message(String::new());
                        callback(terminal, &entered_text, key);
                        return None;
                    }
                    Key::Backspace | Key::Delete => {
                        let _ = entered_text.pop();
                    }
                    Key::Control(Some(c)) if ctrl_key('h', c as u32) => {
                        let _ = entered_text.pop();
                    }
                    Key::Control(Some(c)) if ctrl_key('q', c as u32) => {
                        terminal.set_status_message(String::new());
                        callback(terminal, &entered_text, key);
                        return None;
                    }
                    _ => {}
                }

                callback(terminal, &entered_text, key);
            }
        }
        Some(entered_text)
    }

    fn preprocess_cmd(&self, terminal: &mut Terminal<'_>, cmd: Cmd) {
        use crate::commands::Cmd::*;

        if cmd == Save && !terminal.has_filename() {
            if let Some(filename) = self.prompt(
                terminal,
                "Save as:",
                "(ESC to cancel)",
                |_, _, _| {},
            ) {
                terminal.set_filename(filename);
            }
        } else if cmd == Search {
            let saved_cx = terminal.cursor_x;
            let saved_cy = terminal.cursor_y;
            let saved_col_offset = terminal.col_offset;
            let saved_row_offset = terminal.row_offset;
            let mut last_match = None;
            let mut direction = SearchDirection::Forwards;
            let mut run_search = true;
            let search_str = format!("Search ({}):", direction);
            terminal.debug(format!(
                "cmd == Search. last_match = {:?}, direction = {}\r\n",
                last_match, direction
            ));

            let query = self.prompt(
                terminal,
                &search_str,
                "(Use ESC/Arrows/Enter)",
                |terminal, text, key| {
                    match key {
                        Key::ArrowLeft | Key::ArrowUp => {
                            direction = SearchDirection::Backwards;
                        }
                        Key::ArrowRight | Key::ArrowDown => {
                            direction = SearchDirection::Forwards;
                        }
                        Key::Return | Key::Escape => {
                            direction = SearchDirection::Forwards;
                            last_match = None;
                            run_search = false;
                        }
                        Key::Other(c) if c.is_control() => {}
                        Key::Control(_) => {}
                        _ => {
                            direction = SearchDirection::Forwards;
                            last_match = None;
                        }
                    };
                    if run_search {
                        last_match =
                            terminal.search_for(last_match, direction, text);
                    }
                },
            );

            if query.is_none() {
                terminal.cursor_x = saved_cx;
                terminal.cursor_y = saved_cy;
                terminal.col_offset = saved_col_offset;
                terminal.row_offset = saved_row_offset;
            }

            terminal.clear_search_overlay();
        }
    }

    fn process_keypress(&self, mut terminal: &mut Terminal<'_>) {
        if let Some(key) = self.read_a_character() {
            if let Some(cmd) = terminal.key_to_cmd(key) {
                self.preprocess_cmd(&mut terminal, cmd);
                terminal.process_cmd(cmd);
            }
        }
    }
}
