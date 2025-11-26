use crate::sync::file_data::FileData;
use anyhow::{Context, Result};
use octocrab::{Error as OctocrabError, GitHubError};
use tracing::{info, instrument};

#[derive(Debug)]
pub struct Client {
    client: octocrab::Octocrab,
    gist_id: String,
}

impl Client {
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()
            .with_context(|| "Failed to build the Github client")?;

        Ok(Self { client, gist_id })
    }

    #[instrument(skip_all)]
    pub async fn sync_file(&self, data: FileData) -> Result<(), Error> {
        self.client
            .gists()
            .update(&self.gist_id)
            .file(&data.filename)
            .with_content(&data.body)
            .send()
            .await?;

        info!("File synced: {}", data.path.display());

        Ok(())
    }
}

pub enum Error {
    Github(GitHubError),
    Internal(Box<dyn std::error::Error + Send + Sync>),
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
