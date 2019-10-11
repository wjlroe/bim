use crate::keycodes::Key;
use crate::row::Row;

// Marker for what to do when the prompt comes back
#[derive(Copy, Clone)]
pub enum PromptAction {
    SaveFile,
}

#[derive(PartialEq)]
pub struct Prompt<'a> {
    pub row: Row<'a>,
    prompt_length: usize,
    pub grab_cursor: bool,
    pub finished: bool,
    pub cancelled: bool,
}

fn ensure_prompt_ending(prompt: &str) -> String {
    if !prompt.trim().ends_with(":") {
        let mut new_prompt = String::from(prompt);
        new_prompt.push_str(": ");
        new_prompt
    } else {
        String::from(prompt)
    }
}

impl<'a> Prompt<'a> {
    pub fn new(prompt: &str, grab_cursor: bool) -> Self {
        let new_prompt = ensure_prompt_ending(prompt);
        Self {
            row: Row::new_wo_syntax(&new_prompt),
            prompt_length: new_prompt.len(),
            grab_cursor,
            finished: false,
            cancelled: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.finished && !self.cancelled
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn input(&self) -> &str {
        &self.row.as_str()[self.prompt_length..].trim_matches(char::is_control)
    }

    pub fn type_char(&mut self, typed_char: char) {
        self.row.append_char(typed_char);
    }

    pub fn done(&mut self) {
        self.finished = true;
    }

    pub fn handle_key(&mut self, key: Key) -> bool {
        let mut handled = false;

        match key {
            Key::Other(typed_char) => {
                self.row.append_char(typed_char);
                handled = true;
            }
            Key::Backspace | Key::Delete => {
                self.row.pop_char();
                handled = true;
            }
            Key::Return => {
                self.finished = true;
                handled = true;
            }
            Key::Escape => {
                self.cancelled = true;
                handled = true;
            }
            _ => {}
        }

        handled
    }

    pub fn as_string(&self) -> &str {
        &self.row.render.trim_matches(char::is_control)
    }
}

#[test]
fn test_prompt() {
    let mut prompt = Prompt::new("Save file as", true);
    assert_eq!("Save file as: ", prompt.as_string());
    assert_eq!("", prompt.input());
    prompt.handle_key(Key::Other('h'));
    assert_eq!("Save file as: h", prompt.as_string());
    assert_eq!("h", prompt.input());
}
