use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{sync::Client, watching::Store};

#[derive(Debug)]
pub struct AppState {
    pub watcher_store: Arc<Mutex<Store>>,
    pub _sync_client: Arc<Mutex<Client>>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        // TODO: get token from config
        let sync_client = Arc::new(Mutex::new(Client::new("dummy-token")?));
        let watcher_store = Arc::new(Mutex::new(Store::new(Arc::clone(&sync_client))?));

        Ok(Self {
            watcher_store,
            _sync_client: sync_client,
        })
    }
}
