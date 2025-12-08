use std::{fs, io::stdin};

#[cfg(test)]
use crate::test_support::zed_paths;
use anyhow::{Result, anyhow, bail};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
#[cfg(not(test))]
use paths as zed_paths;
use rpassword::read_password;
use serde::Deserialize;
use zed_extension_api::serde_json::from_value;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub gist_id: String,
    pub github_token: String,
}

impl Config {
    pub fn from_file() -> Result<Self> {
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

    pub fn from_user_input() -> Result<Self> {
        println!("Enter your Github token:");
        let mut github_token: String;

        github_token = read_password()?;
        while github_token.is_empty() {
            println!("Github token cannot be empty");
            github_token = read_password()?;
        }

        println!("Enter your Gist ID:");
        let mut gist_id = String::default();
        stdin().read_line(&mut gist_id)?;
        while gist_id.is_empty() {
            println!("Gist ID cannot be empty");
            stdin().read_line(&mut gist_id)?;
        }
        gist_id = gist_id.trim_end().to_owned();

        Ok(Config {
            github_token,
            gist_id,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use assert_fs::prelude::*;

    use crate::test_support::zed_config_file;

    use super::*;

    #[tokio::test]
    async fn test_from_file_successfully_reads_correct_config_structure() -> Result<()> {
        zed_config_file().write_str(
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

        let config = Config::from_file().expect("Failed to read config from file");

        assert_eq!(config.github_token, "your_github_token");
        assert_eq!(config.gist_id, "your_gist_id");

        Ok(())
    }
}
