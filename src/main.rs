use std::{
    fs,
    io::{self, Write, stdin, stdout},
};

use anyhow::Result;
use clap::{Parser, Subcommand};
#[cfg(test)]
use common::test_support::zed_paths;
#[cfg(not(test))]
use paths as zed_paths;

use common::{config::Config, sync::Client};

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
                Config::from_settings_file()?
            } else {
                println!("Zed settings file not found, probably you haven't installed Zed yet?");
                let mut stdin = io::stdin().lock();
                let mut stdout = io::stdout().lock();
                Config::from_interactive_io(&mut stdin, &mut stdout)?
            };

            let client = Client::new(config.gist_id, config.github_token)?;

            load(&client, force).await?;
        }
    };

    println!("🟢 All done.");

    Ok(())
}

async fn load(client: &Client, force: bool) -> Result<()> {
    for file_load_result in client.load_files().await? {
        match file_load_result {
            Ok((file_name, content)) => process_loaded_file(file_name, content, force)?,
            Err(e) => println!("🔴 {}", e),
        }
    }

    Ok(())
}

fn process_loaded_file(file_name: String, content: String, force: bool) -> Result<()> {
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
            return Ok(());
        }
    }

    fs::write(file_path, content)?;

    println!("Written {file_name}");

    Ok(())
}
