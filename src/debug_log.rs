use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use time::now;

pub struct DebugLog<'a> {
    filename: &'a str,
}

impl<'a> DebugLog<'a> {
    pub fn new(filename: &'a str) -> Self {
        Self { filename }
    }

    pub fn debugln_timestamped(&self, text: &str) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.filename)?;
        let now = now();
        file.write(&format!("{}: ", now.rfc822()).as_bytes())?;
        file.write(text.as_bytes())?;
        file.write("\n".as_bytes())?;
        file.flush()?;
        Ok(())
    }

    pub fn start(&self) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.filename)?;
        file.write(&format!("---\n").as_bytes())?;
        file.flush()?;
        Ok(())
    }
}
