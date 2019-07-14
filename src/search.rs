use crate::commands::SearchDirection;

#[derive(Clone, Default, PartialEq)]
pub struct Search {
    needle: String,
    direction: SearchDirection,
    last_match: Option<(usize, usize)>,
    run_search: bool,
    restore_cursor: bool,
    saved_row_offset: f32,
    saved_col_offset: f32,
}

impl Search {
    pub fn new(saved_row_offset: f32, saved_col_offset: f32) -> Self {
        let mut search = Self::default();
        search.run_search = true;
        search.saved_col_offset = saved_col_offset;
        search.saved_row_offset = saved_row_offset;
        search
    }

    pub fn as_string(&self) -> String {
        format!("Search ({}): {}", self.direction, self.needle)
    }

    pub fn last_match(&self) -> Option<(usize, usize)> {
        self.last_match
    }

    pub fn direction(&self) -> SearchDirection {
        self.direction
    }

    pub fn run_search(&self) -> bool {
        self.run_search
    }

    pub fn needle(&self) -> &str {
        &self.needle
    }

    pub fn restore_cursor(&self) -> bool {
        self.restore_cursor
    }

    pub fn saved_col_offset(&self) -> f32 {
        self.saved_col_offset
    }

    pub fn saved_row_offset(&self) -> f32 {
        self.saved_row_offset
    }

    pub fn stop(&mut self, restore_cursor: bool) {
        self.run_search = false;
        self.restore_cursor = restore_cursor;
    }

    pub fn go_forwards(&mut self) {
        self.direction = SearchDirection::Forwards;
    }

    pub fn go_backwards(&mut self) {
        self.direction = SearchDirection::Backwards;
    }

    pub fn push_char(&mut self, character: char) {
        self.needle.push(character);
        self.last_match = None;
    }

    pub fn del_char(&mut self) {
        if self.needle.pop().is_some() {
            self.last_match = None;
        } else {
            self.run_search = false;
        }
    }

    pub fn set_last_match(&mut self, last_match: Option<(usize, usize)>) {
        self.last_match = last_match;
    }
}