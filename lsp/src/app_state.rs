use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{sync::Client, watching::Store};

#[derive(Debug)]
pub struct AppState {
    pub watcher_store: Arc<Store>,
}

impl AppState {
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let sync_client = Arc::new(Mutex::new(Client::new(gist_id, github_token)?));
        let watcher_store = Arc::new(Store::new(Arc::clone(&sync_client))?);

        Ok(Self { watcher_store })
    }
}
