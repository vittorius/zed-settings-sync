// TODO: consider extracting this module into a separate local crate
// TODO: split into submodules

#![allow(clippy::pedantic)]
#![allow(clippy::expect_used)]

use assert_fs::{TempDir, fixture::ChildPath, prelude::PathChild};
use std::{
    io::{self, BufRead, Cursor, Seek, Write},
    sync::{LazyLock, Mutex},
};

use crate::{ZED_CONFIG_FILE_NAME, interactive_io::InteractiveIO};

static ZED_CONFIG_DIR: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("Failed to create temporary Zed config directory"));
static READ_PASSWORD_INPUTS_REVERSED: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(vec![FAKE_GITHUB_TOKEN.to_string(), String::new()]));

pub const FAKE_GITHUB_TOKEN: &str = "gho_1234567890";

pub struct CursorInteractiveIO<'a> {
    input: Cursor<&'a str>,
    output: Cursor<Vec<u8>>,
}

impl<'a> CursorInteractiveIO<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: Cursor::new(input),
            output: Cursor::new(Vec::new()),
        }
    }

    pub fn rewind_output(&mut self) -> io::Result<()> {
        self.output.rewind()
    }

    pub fn output_lines(self) -> impl Iterator<Item = Result<String, io::Error>> {
        self.output.lines()
    }
}

impl InteractiveIO for CursorInteractiveIO<'_> {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        self.input.read_line(buf)
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        self.output.write_all(line.as_bytes())?;
        self.output.write_all(b"\n")?;
        Ok(())
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        self.output.write_all(text.as_bytes())
    }
}

pub fn zed_config_dir() -> &'static TempDir {
    &ZED_CONFIG_DIR
}

pub fn zed_settings_file() -> ChildPath {
    zed_config_dir().child(*ZED_CONFIG_FILE_NAME)
}

// must be called from a single test function,
// the number of calls is limited to the number of passwords in the input vector
pub fn read_password() -> io::Result<String> {
    Ok(READ_PASSWORD_INPUTS_REVERSED
        .lock()
        .expect("Failed to lock password inputs")
        .pop()
        .expect("No more passwords available from the input"))
}

pub mod zed_paths {
    use std::path::PathBuf;

    use super::{ZED_CONFIG_DIR, ZED_CONFIG_FILE_NAME};

    pub fn settings_file() -> PathBuf {
        ZED_CONFIG_DIR.path().join(*ZED_CONFIG_FILE_NAME)
    }

    pub fn config_dir() -> PathBuf {
        ZED_CONFIG_DIR.path().to_owned()
    }
}

#[macro_export]
macro_rules! nextest_only {
    () => {
        #[ctor::ctor]
        fn check_nextest() {
            if std::env::var("NEXTEST").is_err() {
                eprintln!("ERROR: These tests are designed for Nextest runner");
                eprintln!("Run with: cargo nextest run");
                std::process::exit(1);
            }
        }
    };
}
