use crate::config::RunConfig;

pub struct Options {
    pub no_quit_warning: bool,
    pub run_type: RunConfig,
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
            run_type: RunConfig::default(),
        }
    }
}
