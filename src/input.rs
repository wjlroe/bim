use crate::keycodes::Key;
use crate::prompt::{Prompt, PromptAction};
use crate::row::Row;

pub struct Input<'a> {
    prompt: Prompt<'a>,
    pub next_action: PromptAction,
}

impl<'a> Input<'a> {
    pub fn new(prompt: &str, next_action: PromptAction, grab_cursor: bool) -> Self {
        Self {
            prompt: Prompt::new(prompt, grab_cursor),
            next_action,
        }
    }

    pub fn new_save_file_input(prompt: &str, grab_cursor: bool) -> Self {
        Self::new(prompt, PromptAction::SaveFile, grab_cursor)
    }

    pub fn handle_key(&mut self, key: Key) -> bool {
        self.prompt.handle_key(key)
    }

    pub fn type_char(&mut self, typed_char: char) {
        self.prompt.type_char(typed_char);
    }

    pub fn done(&mut self) {
        self.prompt.done();
    }

    pub fn is_done(&self) -> bool {
        self.prompt.is_done()
    }

    pub fn is_cancelled(&self) -> bool {
        self.prompt.is_cancelled()
    }

    pub fn next_action(&self) -> Option<PromptAction> {
        Some(self.next_action)
    }

    pub fn get_row(&self) -> &Row {
        &self.prompt.row
    }

    pub fn grab_cursor(&self) -> bool {
        self.prompt.grab_cursor
    }

    pub fn display_text(&self) -> &str {
        self.prompt.as_string()
    }

    pub fn input(&self) -> &str {
        self.prompt.input()
    }
}
