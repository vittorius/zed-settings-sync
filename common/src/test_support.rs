// TODO: consider extracting this module into a separate local crate

#![allow(clippy::pedantic)]
#![allow(clippy::expect_used)]

use assert_fs::{TempDir, fixture::ChildPath, prelude::PathChild};
use std::{
    io,
    sync::{LazyLock, Mutex},
};

static ZED_CONFIG_DIR: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("Failed to create temporary Zed config directory"));
static READ_PASSWORD_INPUTS_REVERSED: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(vec![FAKE_GITHUB_TOKEN.to_string(), String::new()]));

const ZED_CONFIG_FILE_NAME: &str = "settings.json";
pub const FAKE_GITHUB_TOKEN: &str = "gho_1234567890";

pub fn zed_config_dir() -> &'static TempDir {
    &ZED_CONFIG_DIR
}

pub fn zed_config_file() -> ChildPath {
    zed_config_dir().child(ZED_CONFIG_FILE_NAME)
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
        ZED_CONFIG_DIR.path().join(ZED_CONFIG_FILE_NAME)
    }

    pub fn config_dir() -> PathBuf {
        ZED_CONFIG_DIR.path().to_owned()
    }
}
