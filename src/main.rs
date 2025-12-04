use std::{fs, io::stdin};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use jsonc_parser::{ParseOptions, cst::CstRootNode, parse_to_serde_value};
use rpassword::read_password;
use serde::Deserialize;
use zed_extension_api::serde_json::from_value;

#[derive(Debug, Deserialize)]
struct Config {
    gist_id: String,
    github_token: String,
}
impl Config {
    fn from_file() -> Result<Self> {
        // we don't care about possible TOCTOU errors because if Zed is installed, its config key is guaranteed to exist
        if !zed_paths::settings_file().try_exists()? {
            bail!("Settings file not found");
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

    fn from_user_input() -> Result<Self> {
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

#[derive(Debug, Parser)]
#[command(about = "Zed Settings Sync extension CLI tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load all Zed user settings files from a gist
    Load,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Load => {
            let config = if zed_paths::settings_file().exists() {
                println!("Loading settings from file");
                Config::from_file()?
            } else {
                println!("Zed settings file not found, probably you haven't installed Zed yet?");
                Config::from_user_input()?
            };

            load(&config).await?;
        }
    };

    println!("🟢 All done.");

    Ok(())
}

async fn load(config: &Config) -> Result<()> {
    // TODO: use the logic to load the gist contents from the shared Client type/module
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(config.github_token.clone())
        .build()
        .with_context(|| "Failed to build the Github client")?;

    let gist = octocrab.gists().get(&config.gist_id).await?;

    for (file_name, file) in gist.files {
        if !file_name.ends_with(".json") && file.content.is_none() {
            continue;
        }

        let mut content = file
            .content
            .expect("File content is already checked for presence");

        if file_name
            == zed_paths::settings_file()
                .file_name()
                .expect("Settings file path must end in valid file name")
                .to_str()
                .expect("Settings file name must be in ASCII")
        {
            let root = CstRootNode::parse(&content, &ParseOptions::default())?;
            let root_obj = root.object_value_or_set();
            root_obj
                .get("lsp")
                .ok_or(anyhow!(r#"Missing "lsp" key"#))?
                .object_value()
                .ok_or(anyhow!(r#"Missing "lsp" configuration object"#))?
                .get("settings_sync")
                .ok_or(anyhow!(r#"Missing "settings_sync" key"#))?
                .object_value()
                .ok_or(anyhow!(r#"Missing "settings_sync" configuration object"#))?
                .get("initialization_options")
                .ok_or(anyhow!(r#"Missing "initialization_options" key"#))?
                .object_value()
                .ok_or(anyhow!(
                    r#"Missing "initialization_options" configuration object"#
                ))?
                .get("github_token")
                .ok_or(anyhow!("Missing github_token"))?
                .set_value(config.github_token.clone().into());
            content = root.to_string();
        }

        // FIXME: prompt on overwrite if exists; use -f option to force

        fs::write(zed_paths::config_dir().join(&file_name), content)?;

        println!("🟢 Successfully written {file_name} file");
    }

    Ok(())
}
