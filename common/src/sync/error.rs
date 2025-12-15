use jsonc_parser::errors::ParseError;
use octocrab::{Error as OctocrabError, GitHubError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid JSON: {0}")]
    InvalidJson(ParseError),
    #[error("Invalid config structure: {0}")]
    InvalidConfig(String),
    #[error("Github error: {0}")]
    Github(GitHubError),
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unhandled internal error from underlying client library: {0}")]
    UnhandledInternal(String),
}

impl From<OctocrabError> for Error {
    fn from(err: OctocrabError) -> Self {
        match err {
            OctocrabError::GitHub { source, .. } => Error::Github(*source),
            OctocrabError::UriParse { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Uri { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Installation { .. } => Error::Internal(Box::new(err)),
            OctocrabError::InvalidHeaderValue { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Http { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::InvalidUtf8 { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Encoder { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Service { source, .. } | OctocrabError::Other { source, .. } => {
                Error::Internal(source)
            }
            OctocrabError::Hyper { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::SerdeUrlEncoded { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Serde { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Json { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::JWT { source, .. } => Error::Internal(Box::new(source)),
            _ => Error::UnhandledInternal(format!(
                "Unhandled Octocrab error from non-exhaustive match, ping author to update deps: {err:?}"
            )),
        }
    }
}
