use Result as StdResult;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};
use tower_lsp::lsp_types::Url;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WatchedPath(PathBuf);

impl WatchedPath {
    pub fn new(file_uri: &Url) -> StdResult<Self, WatchedPathError> {
        if !file_uri.scheme().eq_ignore_ascii_case("file") {
            return Err(WatchedPathError::WrongFileUriFormat);
        }

        let file_path = file_uri
            .to_file_path()
            .map_err(|()| WatchedPathError::WrongFileUriFormat)?;

        match file_path.extension() {
            Some(ext) => {
                if ext != "json" {
                    return Err(WatchedPathError::NotZedConfigFile);
                }
            }
            _ => {
                return Err(WatchedPathError::NotZedConfigFile);
            }
        }

        Ok(Self(Self::build_watched_path(&file_path)?))
    }

    fn build_watched_path(file_path: &Path) -> StdResult<PathBuf, WatchedPathError> {
        if file_path.starts_with(zed_paths::config_dir()) {
            Ok(file_path.to_owned())
        } else if let Some(local_config_dir_path) = file_path.parent()
            && let Some(local_config_dir_name) = local_config_dir_path.file_name()
            && local_config_dir_name == zed_paths::local_settings_folder_name()
        {
            Ok(local_config_dir_path
                .parent()
                .ok_or(WatchedPathError::MissingZedConfigDirParent)?
                .to_owned())
        } else {
            Err(WatchedPathError::NotZedConfigFile)
        }
    }
}

impl Display for WatchedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl AsRef<Path> for WatchedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

pub enum WatchedPathError {
    NotZedConfigFile,
    // error getting parent dir of local zed config (.zed) dir
    MissingZedConfigDirParent,
    WrongFileUriFormat,
}
