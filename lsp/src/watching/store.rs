use std::{collections::HashSet, pin::Pin, sync::Arc};

use anyhow::Result;
use anyhow::bail;
use tokio::sync::Mutex;

use crate::{
    sync::Client,
    watching::{EventHandler, PathWatcher, WatchedPath},
};

#[derive(Debug)]
struct WatchedSet {
    paths: HashSet<WatchedPath>,
    watcher: PathWatcher,
}

impl WatchedSet {
    fn new(event_handler: EventHandler) -> Result<Self> {
        let watcher = PathWatcher::new(event_handler)?;

        Ok(Self {
            paths: HashSet::new(),
            watcher,
        })
    }
}

#[derive(Debug)]
pub struct Store {
    // behind mutex to control the simultaneous change of paths set and path watcher
    watched_set: Mutex<WatchedSet>,
}

impl Store {
    pub fn new(client: Arc<Mutex<Client>>) -> Result<Self> {
        let event_handler = Box::new(move |event| {
            let client_clone = Arc::clone(&client);
            // TODO: construct client-friendly event that contains the actual file path, especially for local config dirs being monitored
            Box::pin(async move {
                // TODO: handle error
                let _ = client_clone.lock().await.notify(event).await;
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        Ok(Self {
            watched_set: Mutex::new(WatchedSet::new(event_handler)?),
        })
    }

    pub async fn watch(&mut self, file_path: WatchedPath) -> anyhow::Result<()> {
        {
            let mut watched_set = self.watched_set.lock().await;

            if watched_set.paths.contains(&file_path) {
                bail!("Path is already being watched: {file_path}");
            }

            watched_set.watcher.watch(&file_path)?;
            watched_set.paths.insert(file_path);
        }

        Ok(())
    }

    pub async fn unwatch(&mut self, file_path: WatchedPath) -> anyhow::Result<()> {
        {
            let mut watched_set = self.watched_set.lock().await;

            if !watched_set.paths.contains(&file_path) {
                bail!("Path is not being watched, failed to unwatch: {file_path}");
            }

            watched_set.watcher.unwatch(&file_path)?;
            watched_set.paths.remove(&file_path);
        }

        Ok(())
    }

    // no separate "start" method as watcher is started immediately when it's created
    // no need to stop watcher or clear the store because it will be stopped (when dropped) on the store drop
}
