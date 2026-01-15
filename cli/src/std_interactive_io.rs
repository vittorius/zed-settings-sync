use std::io::{self, Write};

use common::interactive_io::InteractiveIO;

pub struct StdInteractiveIO;

impl InteractiveIO for StdInteractiveIO {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        io::stdin().read_line(buf)
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut lock = io::stdout().lock();
        lock.write_all(line.as_bytes())?;
        lock.write_all(b"\n")?;

        Ok(())
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        let mut lock = io::stdout().lock();
        lock.write_all(text.as_bytes())?;
        lock.flush()?;

        Ok(())
    }
}
