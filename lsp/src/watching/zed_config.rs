use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use anyhow::Result;
#[cfg(not(test))]
use paths as zed_paths;
#[cfg(test)]
use test_support::zed_paths;
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

#[derive(Debug, PartialEq, Eq)]
pub enum ZedConfigPathError {
    NotZedConfigFile,
    WrongFileUriFormat,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use test_support::zed_paths;
    use tower_lsp::lsp_types::Url;

    use crate::watching::{ZedConfigFilePath, ZedConfigPathError};

    #[test]
    fn test_from_file_uri_success() {
        let file_uri = Url::parse(&format!(
            "file:///{}/tasks.json",
            zed_paths::config_dir().display()
        ))
        .unwrap();
        let config_path = ZedConfigFilePath::from_file_uri(&file_uri).unwrap();

        assert_eq!(
            config_path.path.to_string_lossy(),
            zed_paths::config_dir().join("tasks.json").to_string_lossy()
        );
    }

    #[test]
    fn test_from_file_uri_failure_wrong_format() {
        let file_uri = Url::parse("lol:///home/user/.config/zed/settings.json").unwrap();

        assert_eq!(
            ZedConfigFilePath::from_file_uri(&file_uri).unwrap_err(),
            ZedConfigPathError::WrongFileUriFormat
        );
    }

    #[test]
    fn test_from_file_uri_failure_not_zed_config_file() {
        let file_uri = Url::parse(&format!(
            "file:///{}/settings.kek",
            zed_paths::config_dir().display()
        ))
        .unwrap();

        assert_eq!(
            ZedConfigFilePath::from_file_uri(&file_uri),
            Err(ZedConfigPathError::NotZedConfigFile)
        );
    }

    #[test]
    fn test_to_watched_path_buf_success() {
        let file_uri =
            Url::parse(&format!("file:///{}", zed_paths::settings_file().display())).unwrap();
        let config_path = ZedConfigFilePath::from_file_uri(&file_uri).unwrap();

        assert_eq!(
            config_path.to_watched_path_buf().display().to_string(),
            zed_paths::settings_file().display().to_string()
        );
    }
}
