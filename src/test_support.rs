use assert_fs::{TempDir, fixture::ChildPath, prelude::PathChild};
use std::sync::LazyLock;

static ZED_CONFIG_DIR: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("Failed to create temporary Zed config directory"));

const ZED_CONFIG_FILE_NAME: &str = "settings.json";

pub fn zed_config_dir() -> &'static TempDir {
    &ZED_CONFIG_DIR
}

pub fn zed_config_file() -> ChildPath {
    zed_config_dir().child(ZED_CONFIG_FILE_NAME)
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
