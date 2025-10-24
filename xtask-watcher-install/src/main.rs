use std::{
    env,
    error::Error,
    fs,
    path::Path,
    process::{Command, exit},
};

const EXTENSION_ID: &str = "settings-sync";
const WATCHER_BINARY: &str = "zed-settings-sync-watcher";

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("Building the watcher service...");

    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "-p", WATCHER_BINARY]);
    let status = cmd.status()?;

    if !status.success() {
        eprintln!("failed to build the watcher service");
        eprintln!("failed command: {cmd:?}");
        exit(status.code().unwrap_or(1));
    }

    eprintln!("Done");

    let from = Path::new("target/debug").join(WATCHER_BINARY);
    let to = zed_paths::extensions_dir()
        .join("work")
        .join(EXTENSION_ID)
        .join(WATCHER_BINARY);
    eprintln!(
        "Copying the watcher binary from {} to the extension working directory {}...",
        from.display(),
        to.display()
    );
    fs::copy(from, to)?;

    eprintln!("Done");

    Ok(())
}
