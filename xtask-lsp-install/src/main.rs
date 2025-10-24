use std::{
    env,
    error::Error,
    fs,
    path::Path,
    process::{Command, exit},
};

const EXTENSION_ID: &str = "settings-sync";
const ZED_SETTINGS_SYNC_BINARY: &str = "zed-settings-sync-lsp";

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("Building the LSP server...");

    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "-p", ZED_SETTINGS_SYNC_BINARY]);
    let status = cmd.status()?;

    if !status.success() {
        eprintln!("failed to build the LSP server");
        eprintln!("failed command: {cmd:?}");
        exit(status.code().unwrap_or(1));
    }

    eprintln!("Done");

    let from = Path::new("target/debug").join(ZED_SETTINGS_SYNC_BINARY);
    let to = zed_paths::extensions_dir()
        .join("work")
        .join(EXTENSION_ID)
        .join(ZED_SETTINGS_SYNC_BINARY);
    eprintln!(
        "Copying the LSP binary from {} to the extension working directory {}...",
        from.display(),
        to.display()
    );
    fs::copy(from, to)?;

    eprintln!("Done");

    Ok(())
}
