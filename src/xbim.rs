use bim::config::RunConfig;
use bim::gui::gfx_ui;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let filename_arg = env::args().skip(1).nth(0);
    let run_type = if let Some(filename) = filename_arg {
        RunConfig::RunOpenFile(filename)
    } else {
        RunConfig::Run
    };

    gfx_ui::run(run_type)?;
    Ok(())
}
