use std::sync::LazyLock;

use paths as zed_paths;

pub mod config;
pub mod interactive_io;
pub mod sync;

static ZED_CONFIG_FILE_NAME: LazyLock<&str> = LazyLock::new(|| {
    // currently, it's "settings.json" at won't likely change to some gibberish,
    // so all "expect" calls below are safe
    #[allow(clippy::expect_used)]
    zed_paths::settings_file()
        .file_name()
        .expect(r#"Settings file name from Zed "paths" crate terminates in .."#)
        .to_str()
        .expect(r#"Non UTF-8 settings file name from Zed from Zed "paths" crate"#)
});

#[cfg(test)]
test_support::nextest_only!();
