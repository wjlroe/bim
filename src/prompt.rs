use crate::keycodes::Key;
use crate::search;

pub enum Prompt {
    Search(search::Search),
}

impl Prompt {
    pub fn new_search(saved_row_offset: f32, saved_col_offset: f32) -> Prompt {
        let search = search::Search::new(saved_row_offset, saved_col_offset);
        Prompt::Search(search)
    }

    pub fn handle_key(&mut self, key: Key) -> bool {
        // TODO: maybe a trait method here?
        match self {
            Prompt::Search(search) => search.handle_search_key(key),
        }
    }

    pub fn top_left_string(&self) -> Option<String> {
        // TODO: maybe move into a trait
        match self {
            Prompt::Search(search) => Some(search.as_string()),
        }
    }
}
