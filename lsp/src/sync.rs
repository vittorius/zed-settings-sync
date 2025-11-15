use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::{info, instrument};

pub struct FileData {
    path: PathBuf,
    body: String,
}

impl FileData {
    pub fn new(path: PathBuf, body: String) -> Self {
        Self { path, body }
    }
}

#[derive(Debug)]
pub struct Client {
    _client: octocrab::Octocrab,
}

impl Client {
    pub fn new(personal_token: &str) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(personal_token)
            .build()
            .with_context(|| "Failed to build the Github client")?;
        Ok(Self { _client: client })
    }
}

impl Client {
    #[instrument(skip_all)]
    pub async fn sync_file(&self, data: FileData) -> Result<()> {
        // TODO: replace dummy async block with the REST client call
        async {
            info!("file saved: {}: {}", data.path.display(), data.body);

            Ok(())
        }
        .await
    }
}
