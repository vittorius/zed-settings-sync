use std::fs;

use anyhow::{Result, anyhow, bail};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
#[cfg(not(test))]
use paths as zed_paths;
#[cfg(not(test))]
use rpassword::read_password;
use serde::Deserialize;
#[cfg(test)]
use test_support::read_password;
#[cfg(test)]
use test_support::zed_paths;
use zed_extension_api::serde_json::from_value;

use crate::interactive_io::InteractiveIO;

#[derive(Debug, Deserialize)]
pub struct Config {
    gist_id: String,
    github_token: String,
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
#[cfg_attr(feature = "test-support", mockall::automock)]
impl Config {
    #[must_use]
    pub fn gist_id(&self) -> &str {
        &self.gist_id
    }

    #[must_use]
    pub fn github_token(&self) -> &str {
        &self.github_token
    }

    pub fn from_settings_file() -> Result<Self> {
        // we don't care about possible TOCTOU errors because if Zed is installed, its config key is guaranteed to exist
        if !zed_paths::settings_file().try_exists()? {
            bail!(
                "Settings file not found at: {}",
                zed_paths::settings_file().display()
            );
        }
        let content = fs::read_to_string(zed_paths::settings_file())?;
        let zed_settings = parse_to_serde_value(&content, &ParseOptions::default())?
            .ok_or(anyhow!("Settings file is empty"))?;
        let config = from_value(
            zed_settings
                .pointer("/lsp/settings_sync/initialization_options") // TODO: make this pointer key shared among crates of this package
                .ok_or(anyhow!(
                    "Missing lsp.settings_sync.initialization_options key in settings tree"
                ))?
                .clone(),
        )?;

        Ok(config)
    }

    pub fn from_interactive_io<T: InteractiveIO + 'static>(io: &mut T) -> Result<Self> {
        io.write_line("Enter your Github token:")?;
        let mut github_token: String;

        github_token = read_password()?;
        while github_token.is_empty() {
            io.write_line("Github token cannot be empty")?;
            github_token = read_password()?;
        }

        io.write_line("Enter your Gist ID:")?;
        let mut gist_id = String::default();
        io.read_line(&mut gist_id)?;
        gist_id = gist_id.trim_end().to_owned();

        while gist_id.is_empty() {
            io.write_line("Gist ID cannot be empty")?;
            io.read_line(&mut gist_id)?;
            gist_id = gist_id.trim_end().to_owned();
        }

        Ok(Config {
            gist_id,
            github_token,
        })
    }
}

// NOTE: these tests don't use any cross-thread sync for operations on shared FS paths
// so they must be run sequentially or in parallel processes
// e.g. using cargo nextest or serial-test crate in case of cargo test
#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use std::io::{self, BufRead, Cursor, Seek, Write};

    use assert_fs::prelude::*;
    use test_support::{FAKE_GITHUB_TOKEN, zed_settings_file};

    use super::*;

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

    #[tokio::test]
    async fn test_from_file_success() -> Result<()> {
        zed_settings_file().write_str(
            r#"
            {
                "lsp": {
                    "settings_sync": {
                        "initialization_options": {
                            "github_token": "your_github_token",
                            "gist_id": "your_gist_id"
                        }
                    }
                }
            }
            "#,
        )?;

        let config = Config::from_settings_file().expect("Failed to read config from file");

        assert_eq!(config.github_token, "your_github_token");
        assert_eq!(config.gist_id, "your_gist_id");

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_failure_when_settings_file_is_missing() {
        let config = Config::from_settings_file();

        assert_eq!(
            config.unwrap_err().to_string(),
            format!(
                "Settings file not found at: {}",
                zed_paths::settings_file().display()
            )
        );
    }

    #[tokio::test]
    async fn test_from_file_fails_when_settings_file_is_empty() -> Result<()> {
        zed_settings_file().touch()?;

        let config = Config::from_settings_file();

        assert_eq!(config.unwrap_err().to_string(), "Settings file is empty");

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_fails_when_config_is_missing_lsp_key() -> Result<()> {
        zed_settings_file().write_str("{}")?;

        let config = Config::from_settings_file();

        assert_eq!(
            config.unwrap_err().to_string(),
            "Missing lsp.settings_sync.initialization_options key in settings tree"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_fails_when_config_is_missing_lsp_settings_sync_key() -> Result<()> {
        zed_settings_file().write_str(r#"{"lsp": {}}"#)?;

        let config = Config::from_settings_file();

        assert_eq!(
            config.unwrap_err().to_string(),
            "Missing lsp.settings_sync.initialization_options key in settings tree"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_fails_when_config_is_missing_lsp_settings_sync_initialization_options_key()
    -> Result<()> {
        zed_settings_file().write_str(
            r#"
            {
              "lsp": {
                "settings_sync": {}
              }
            }"#,
        )?;

        let config = Config::from_settings_file();

        assert_eq!(
            config.unwrap_err().to_string(),
            "Missing lsp.settings_sync.initialization_options key in settings tree"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_fails_when_config_is_missing_gist_id_key() -> Result<()> {
        zed_settings_file().write_str(
            r#"
            {
              "lsp": {
                "settings_sync": {
                  "initialization_options": {}
                }
              }
            }"#,
        )?;

        let config = Config::from_settings_file();

        assert_eq!(config.unwrap_err().to_string(), "missing field `gist_id`");

        Ok(())
    }

    #[tokio::test]
    async fn test_from_file_fails_when_config_is_missing_github_token_key() -> Result<()> {
        zed_settings_file().write_str(
            r#"
            {
              "lsp": {
                "settings_sync": {
                  "initialization_options": {
                    "gist_id": "1234567890abcdef"
                  }
                }
              }
            }"#,
        )?;

        let config = Config::from_settings_file();

        assert_eq!(
            config.unwrap_err().to_string(),
            "missing field `github_token`"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_from_user_input_success() -> Result<()> {
        let input_lines = "\nabcdef1234567890\n"; // empty line followed by fake gist id
        let mut io = CursorInteractiveIO::new(input_lines);

        let config = Config::from_interactive_io(&mut io)?;

        io.rewind_output()?;
        let mut output_lines_iter = io.output_lines();

        assert_eq!(
            output_lines_iter.next().unwrap()?,
            "Enter your Github token:"
        );
        assert_eq!(
            output_lines_iter.next().unwrap()?,
            "Github token cannot be empty"
        ); // first input line is empty
        assert_eq!(output_lines_iter.next().unwrap()?, "Enter your Gist ID:");
        assert_eq!(
            output_lines_iter.next().unwrap()?,
            "Gist ID cannot be empty"
        ); // first input line is empty

        assert_eq!(config.github_token, FAKE_GITHUB_TOKEN);
        assert_eq!(config.gist_id, "abcdef1234567890");

        Ok(())
    }
}
