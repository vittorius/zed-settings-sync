use std::sync::Arc;

use anyhow::Result;
use common::sync::GithubClient;
#[cfg(not(test))]
use tower_lsp::Client as LspClient;

#[cfg(test)]
use crate::mocks::MockLspClient as LspClient;
#[cfg(test)]
use crate::watching::MockPathStore as PathStore;
#[cfg(not(test))]
use crate::watching::PathStore;

#[derive(Debug)]
pub struct AppState {
    pub watched_paths: PathStore,
}

impl AppState {
    pub fn new(gist_id: String, github_token: String, lsp_client: Arc<LspClient>) -> Result<Self> {
        let sync_client = Arc::new(GithubClient::new(gist_id, github_token)?);
        let watched_paths = PathStore::new(sync_client, lsp_client)?;

        Ok(Self { watched_paths })
    }
}
