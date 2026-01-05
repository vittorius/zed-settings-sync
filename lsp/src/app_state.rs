use anyhow::Result;
use common::sync::GithubClient;
use std::sync::Arc;

use crate::watching::Store;

#[derive(Debug)]
pub struct AppState {
    pub watcher_store: Arc<Store>,
}

impl AppState {
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let sync_client = Arc::new(GithubClient::new(gist_id, github_token)?);
        let watcher_store = Arc::new(Store::new(sync_client)?);

        Ok(Self { watcher_store })
    }
}
