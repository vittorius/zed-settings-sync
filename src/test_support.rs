pub mod zed_paths {
    use std::path::PathBuf;

    pub fn settings_file() -> PathBuf {
        "/Users/vittorius/.config/zed/settings.json".into()
    }

    pub fn config_dir() -> PathBuf {
        "/Users/vittorius/.config/zed".into()
    }
}
