use anyhow::Result;
use paths as zed_paths;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};
use tower_lsp::lsp_types::Url;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ZedConfigFilePath {
    path: PathBuf,
}

impl ZedConfigFilePath {
    pub fn from_file_uri(file_uri: &Url) -> Result<Self, ZedConfigPathError> {
        if !file_uri.scheme().eq_ignore_ascii_case("file") {
            return Err(ZedConfigPathError::WrongFileUriFormat);
        }

        let path = file_uri
            .to_file_path()
            .map_err(|()| ZedConfigPathError::WrongFileUriFormat)?;

        validate_file_path(&path)?;
        validate_file_extension(&path)?;

        Ok(Self { path })
    }

    pub fn to_watched_path_buf(&self) -> PathBuf {
        self.path.clone()
    }
}

fn validate_file_extension(path: &Path) -> Result<(), ZedConfigPathError> {
    match path.extension() {
        Some(ext) => {
            if ext == "json" {
                Ok(())
            } else {
                Err(ZedConfigPathError::NotZedConfigFile)
            }
        }
        None => Err(ZedConfigPathError::NotZedConfigFile),
    }
}

fn validate_file_path(path: &Path) -> Result<(), ZedConfigPathError> {
    if path.starts_with(zed_paths::config_dir()) {
        Ok(())
    } else {
        Err(ZedConfigPathError::NotZedConfigFile)
    }
}

impl Display for ZedConfigFilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl AsRef<Path> for ZedConfigFilePath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

pub enum ZedConfigPathError {
    NotZedConfigFile,
    WrongFileUriFormat,
}
