use crate::keycodes::Key;
use crate::search;

pub struct InputPrompt {
    prompt: String,
    pub input: String,
    complete: bool,
}

impl InputPrompt {
    fn new(prompt: String) -> Self {
        Self {
            prompt,
            input: String::new(),
            complete: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.complete
    }

    fn handle_key(&mut self, key: Key) -> bool {
        let mut handled = false;

        match key {
            Key::Other(typed_char) => {
                self.input.push(typed_char);
                handled = true;
            }
            Key::Backspace | Key::Delete => {
                self.input.pop();
                handled = true;
            }
            Key::Return => {
                self.complete = true;
                handled = true;
            }
            _ => {}
        }

        handled
    }

    fn as_string(&self) -> String {
        format!("{}: {}", self.prompt, self.input)
    }
}

pub enum Prompt {
    Search(search::Search),
    Input(InputPrompt),
}

impl Prompt {
    pub fn new_search(saved_row_offset: f32, saved_col_offset: f32) -> Prompt {
        let search = search::Search::new(saved_row_offset, saved_col_offset);
        Prompt::Search(search)
    }

    pub fn new_input(prompt: String) -> Prompt {
        Prompt::Input(InputPrompt::new(prompt))
    }

    pub fn handle_key(&mut self, key: Key) -> bool {
        // TODO: maybe a trait method here?
        match self {
            Prompt::Search(search) => search.handle_search_key(key),
            Prompt::Input(input_prompt) => input_prompt.handle_key(key),
        }
    }

    pub fn top_left_string(&self) -> Option<String> {
        // TODO: maybe move into a trait
        match self {
            Prompt::Search(search) => Some(search.as_string()),
            Prompt::Input(input_prompt) => Some(input_prompt.as_string()),
        }
    }
}
