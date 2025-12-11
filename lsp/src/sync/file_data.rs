use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub struct FileData {
    pub path: PathBuf,
    pub filename: String,
    pub body: String,
}

impl FileData {
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
