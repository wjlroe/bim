use commands::Cmd;
use keycodes::{ctrl_key, Key};
use terminal::Terminal;

pub trait Editor {
    fn enable_raw_mode(&self);
    fn get_window_size(&self) -> Terminal;
    fn read_a_character(&self) -> Option<Key>;

    fn prompt(
        &self,
        terminal: &mut Terminal,
        status_left: &str,
        status_right: &str,
    ) -> Option<String> {
        let mut entered_text = String::new();
        loop {
            terminal.set_status_message(
                format!("{} {} {}", status_left, entered_text, status_right),
            );
            terminal.refresh();
            if let Some(key) = self.read_a_character() {
                match key {
                    Key::Other(c) if !c.is_control() => {
                        entered_text.push(c);
                    }
                    Key::Return if !entered_text.is_empty() => {
                        break;
                    }
                    Key::Escape => {
                        terminal.set_status_message(String::new());
                        return None;
                    }
                    Key::Backspace | Key::Delete => {
                        let _ = entered_text.pop();
                    }
                    Key::Other(c) => if ctrl_key('h', c as u32) {
                        let _ = entered_text.pop();
                    },
                    _ => {}
                }
            }
        }
        Some(entered_text)
    }

    fn preprocess_cmd(&self, mut terminal: &mut Terminal, cmd: Cmd) {
        use commands::Cmd::*;

        if cmd == Save && terminal.filename.is_none() {
            if let Some(filename) =
                self.prompt(terminal, "Save as:", "(ESC to cancel)")
            {
                terminal.filename = Some(filename);
            }
        }
    }

    fn process_keypress(&self, mut terminal: &mut Terminal) {
        if let Some(key) = self.read_a_character() {
            if let Some(cmd) = terminal.key_to_cmd(key) {
                self.preprocess_cmd(&mut terminal, cmd);
                terminal.process_cmd(cmd);
            }
        }
    }
}
