use std::{
    fs,
    io::{self, Write, stdin, stdout},
};

#[cfg(test)]
use crate::test_support::zed_paths;
use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use jsonc_parser::{ParseOptions, cst::CstRootNode};
#[cfg(not(test))]
use paths as zed_paths;

use crate::config::Config;

mod config;
#[cfg(test)]
mod test_support;

#[derive(Debug, Parser)]
#[command(about = "Zed Settings Sync extension CLI tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load all Zed user settings files from a gist
    Load {
        /// Force overwriting local settings files even if they exist
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Load { force } => {
            let config = if zed_paths::settings_file().exists() {
                println!("Loading settings from file");
                Config::from_file()?
            } else {
                println!("Zed settings file not found, probably you haven't installed Zed yet?");
                let mut stdin = io::stdin().lock();
                let mut stdout = io::stdout().lock();
                Config::from_user_input(&mut stdin, &mut stdout)?
            };

            load(&config, force).await?;
        }
    };

    println!("🟢 All done.");

    Ok(())
}

async fn load(config: &Config, force: bool) -> Result<()> {
    // TODO: use the logic to load the gist contents from the shared Client type/module
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(config.github_token.clone())
        .build()
        .with_context(|| "Failed to build the Github client")?;

    let gist = octocrab.gists().get(&config.gist_id).await?;

    for (file_name, file) in gist.files {
        if !file_name.ends_with(".json") || file.content.is_none() {
            continue;
        }

        let file_path = zed_paths::config_dir().join(&file_name);

        if file_path.exists() && !force {
            print!("🟡 {file_name} exists, overwrite (y/n)? ");
            stdout().flush()?;

            let mut answer = String::new();
            stdin().read_line(&mut answer)?;

            if answer.trim().to_lowercase().starts_with('y') {
                println!("🔴 Overwriting {file_name}...");
            } else {
                println!("Skipping {file_name}");
                continue;
            }
        }

        let mut content = file
            .content
            .expect("File content is already checked for presence");

        let settings_file_name = zed_paths::settings_file();
        let settings_file_name = settings_file_name
            .file_name()
            .with_context(|| {
                format!(
                    "Settings file path ends with invalid file name: {}",
                    zed_paths::settings_file().display()
                )
            })?
            .to_string_lossy();

        if file_name == settings_file_name {
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

        fs::write(file_path, content)?;

        println!("Written {file_name}");
    }

    Ok(())
}
