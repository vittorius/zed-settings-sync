use std::fs;

use crate::interactive_io::InteractiveIO;
#[cfg(test)]
use crate::test_support::CursorInteractiveIO;
#[cfg(test)]
use crate::test_support::read_password;
#[cfg(test)]
use crate::test_support::zed_paths;
use anyhow::{Result, anyhow, bail};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use mockall::automock;
#[cfg(not(test))]
use paths as zed_paths;
#[cfg(not(test))]
use rpassword::read_password;
use serde::Deserialize;
use zed_extension_api::serde_json::from_value;

#[derive(Debug, Deserialize)]
pub struct Config {
    gist_id: String,
    github_token: String,
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
#[automock]
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
    use assert_fs::prelude::*;

    use crate::test_support::{FAKE_GITHUB_TOKEN, zed_settings_file};

    use super::*;

    #[tokio::test]
    async fn test_from_file_successfully_reads_correct_config_structure() -> Result<()> {
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
    async fn test_from_file_fails_when_settings_file_is_missing() {
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
    async fn test_from_user_input_successfully_reads_config() -> Result<()> {
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
