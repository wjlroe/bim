pub const TAB_STOP: usize = 8;
pub const BIM_QUIT_TIMES: i8 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunConfig {
    Debug,
    RunOpenFiles(Vec<String>),
    Run,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig::Run
    }
}
