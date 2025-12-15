use std::io;

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub trait InteractiveIO {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize>;
    fn write_line(&mut self, line: &str) -> io::Result<()>;
    fn write(&mut self, text: &str) -> io::Result<()>;
}
