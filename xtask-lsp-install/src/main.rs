use std::{
    env,
    error::Error,
    fs,
    path::Path,
    process::{Command, exit},
};

// TODO: extract to a shared local "paths" crate
const EXTENSION_ID: &str = "settings-sync";
const LSP_BINARY: &str = "zed-settings-sync-lsp";

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("Building the LSP server...");

    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "-p", LSP_BINARY]);
    let status = cmd.status()?;

    if !status.success() {
        eprintln!("failed to build the LSP server");
        eprintln!("failed command: {cmd:?}");
        exit(status.code().unwrap_or(1));
    }

    eprintln!("Done");

    let from = Path::new("target/debug").join(LSP_BINARY);
    let to = zed_paths::extensions_dir()
        .join("work")
        .join(EXTENSION_ID)
        .join(LSP_BINARY);
    let to = fs::canonicalize(to)?;

    eprintln!(
        "Copying the LSP binary from {} to the extension working directory {}...",
        from.display(),
        to.display()
    );
    fs::remove_file(&to)?; // have to remove symlink target otherwise the copied binary is broken for an unknown reason
    fs::copy(from, to)?;

    eprintln!("Done");

    Ok(())
}
