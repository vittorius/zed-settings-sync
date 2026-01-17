use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
pub use github_client::*;
use thiserror::Error;

use crate::sync::{Error, LocalFileData};

mod github_client;

pub type FileResult = Result<(String, String), FileError>;

#[async_trait]
pub trait Client: Send + Sync {
    #[allow(clippy::missing_errors_doc)]
    #[allow(
        async_fn_in_trait,
        reason = "This trait is intended to be used by zed-settings-sync crate only"
    )]
    async fn sync_file(&self, data: LocalFileData) -> Result<(), FileError>;

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    #[allow(
        async_fn_in_trait,
        reason = "This trait is intended to be used by zed-settings-sync crate only"
    )]
    async fn load_files(&self) -> Result<Box<dyn Iterator<Item = FileResult>>, Error>;
}

#[derive(Error, Debug)]
#[error("Error syncing file {file_name}: {error}")]
pub struct FileError {
    file_name: String,
    error: Error,
}

impl FileError {
    pub fn from_error(file_name: impl Into<String>, error: Error) -> Self {
        Self {
            file_name: file_name.into(),
            error,
        }
    }
}
