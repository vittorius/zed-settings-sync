use std::fmt::Debug;

use crate::sync::{Error, LocalFileData};
use anyhow::Result;
use async_trait::async_trait;
pub use github_client::*;
use thiserror::Error;

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
    async fn load_files<'a>(&'a self) -> Result<Box<dyn Iterator<Item = FileResult> + 'a>, Error>;
}

#[derive(Error, Debug)]
#[error("Error processing file {file_name}: {error}")]
pub struct FileError {
    file_name: String,
    error: Error,
}

impl FileError {
    fn from_error(file_name: impl Into<String>, error: Error) -> Self {
        Self {
            file_name: file_name.into(),
            error,
        }
    }
}
