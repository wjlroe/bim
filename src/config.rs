#[derive(PartialEq, Eq)]
pub enum RunConfig {
    Debug,
    RunOpenFile(String),
    Run,
}
