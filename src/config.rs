pub const TAB_STOP: usize = 8;

#[derive(PartialEq, Eq)]
pub enum RunConfig {
    Debug,
    RunOpenFile(String),
    Run,
}
