use std::path::PathBuf;

use anyhow::{Result, anyhow};

pub struct LocalFileData {
    pub path: PathBuf,
    pub filename: String,
    pub body: String,
}

impl LocalFileData {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(path: PathBuf, body: String) -> Result<Self> {
        Ok(Self {
            filename: path
                .file_name()
                .ok_or(anyhow!(
                    "Path terminates in .., should be impossible for a Zed config file path"
                ))?
                .to_string_lossy()
                .into_owned(),
            path,
            body,
        })
    }
}
