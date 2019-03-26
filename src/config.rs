pub const TAB_STOP: usize = 8;
pub const BIM_QUIT_TIMES: i8 = 3;

#[derive(PartialEq, Eq)]
pub enum RunConfig {
    Debug,
    RunOpenFile(String),
    Run,
}
