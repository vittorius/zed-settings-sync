use std::sync::Arc;

use anyhow::Result;
use common::sync::GithubClient;

use crate::watching::PathStore;

#[derive(Debug)]
pub struct AppState {
    pub watcher_store: Arc<PathStore>,
}

impl AppState {
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let sync_client = Arc::new(GithubClient::new(gist_id, github_token)?);
        let watcher_store = Arc::new(PathStore::new(sync_client)?);

        Ok(Self { watcher_store })
    }
}
