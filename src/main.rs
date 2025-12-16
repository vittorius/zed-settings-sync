use anyhow::Result;
use clap::{Parser, Subcommand};
#[cfg(test)]
use common::test_support::zed_paths;
#[cfg(not(test))]
use paths as zed_paths;

use common::{config::Config, sync::Client};

use crate::std_interactive_io::StdInteractiveIO;
use crate::sync::Loader;

mod std_interactive_io;
mod sync;

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
    let mut std_io = StdInteractiveIO;

    match args.command {
        Command::Load { force } => {
            let config = if zed_paths::settings_file().exists() {
                println!("Loading settings from file");
                Config::from_settings_file()?
            } else {
                println!("Zed settings file not found, probably you haven't installed Zed yet?");
                Config::from_interactive_io(&mut std_io)?
            };

            let client = Client::new(config.gist_id, config.github_token)?;
            let mut loader = Loader::new(&client, &mut std_io, force);

            loader.load_files().await?;
        }
    };

    println!("🟢 All done.");

    Ok(())
}
