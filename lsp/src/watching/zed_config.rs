use anyhow::Result;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};
use tower_lsp::lsp_types::Url;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ZedConfigFilePath {
    path: PathBuf,
    kind: Kind,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Kind {
    Global,
    Local,
}

impl ZedConfigFilePath {
    pub fn from_file_uri(file_uri: &Url) -> Result<Self, ZedConfigPathError> {
        if !file_uri.scheme().eq_ignore_ascii_case("file") {
            return Err(ZedConfigPathError::WrongFileUriFormat);
        }

        let path = file_uri
            .to_file_path()
            .map_err(|()| ZedConfigPathError::WrongFileUriFormat)?;
        let kind = get_kind(&path)?;

        Ok(Self { path, kind })
    }

    pub fn is_valid(path: &Path) -> bool {
        validate_file_extension(path).is_ok() && get_kind(path).is_ok()
    }

    pub fn to_watched_path_buf(&self) -> PathBuf {
        match self.kind {
            Kind::Global => self.path.clone(),
            Kind::Local => self
                .path
                .parent()
                .expect("Parent dir of local settings dir does not exist")
                .to_owned(),
        }
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

fn get_kind(path: &Path) -> Result<Kind, ZedConfigPathError> {
    if path.starts_with(zed_paths::config_dir()) {
        Ok(Kind::Global)
    } else if let Some(local_config_dir_path) = path.parent()
        && let Some(local_config_dir_name) = local_config_dir_path.file_name()
        && local_config_dir_name == zed_paths::local_settings_folder_name()
    {
        Ok(Kind::Local)
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
