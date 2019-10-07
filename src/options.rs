use crate::config::RunConfig;
use crate::keymap::{Keymap, DEFAULT_KEYMAP};

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub no_quit_warning: bool,
    pub vsplit: bool,
    pub run_type: RunConfig,
    pub keymap: Keymap,
}

impl Options {
    pub fn show_quit_warning(&self) -> bool {
        !self.no_quit_warning
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            no_quit_warning: false,
            vsplit: false,
            run_type: RunConfig::default(),
            keymap: DEFAULT_KEYMAP.clone(),
        }
    }
}
