use crate::{
    sync::{Client, Error as SyncError, FileData},
    watching::{EventHandler, PathWatcher},
};
use anyhow::Result;
use anyhow::{Context, anyhow, bail};
use notify::{Event, EventKind, event::ModifyKind};
use std::{collections::HashSet, fs, path::PathBuf, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error};

#[derive(Debug)]
struct WatchedSet {
    paths: HashSet<PathBuf>,
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
    pub fn new(client: Arc<Client>) -> Result<Self> {
        let event_handler = Box::new(move |event| {
            let client_clone = Arc::clone(&client);
            Box::pin(async move {
                match process_event(&event) {
                    Ok(data) => {
                        let Some(data) = data else {
                            return;
                        };

                        if let Err(err) = client_clone.sync_file(data).await {
                            log_sync_error(err);
                        }
                    }
                    Err(err) => error!("Could not process file event: {err}"),
                }
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        Ok(Self {
            watched_set: Mutex::new(WatchedSet::new(event_handler)?),
        })
    }

    pub async fn watch(&self, file_path: PathBuf) -> anyhow::Result<()> {
        {
            let mut watched_set = self.watched_set.lock().await;

            if watched_set.paths.contains(&file_path) {
                bail!("Path is already being watched: {}", file_path.display());
            }

            watched_set.watcher.watch(&file_path)?;
            watched_set.paths.insert(file_path);
        }

        Ok(())
    }

    pub async fn unwatch(&self, file_path: PathBuf) -> anyhow::Result<()> {
        {
            let mut watched_set = self.watched_set.lock().await;

            if !watched_set.paths.contains(&file_path) {
                bail!(
                    "Path is not being watched, failed to unwatch: {}",
                    file_path.display()
                );
            }

            watched_set.watcher.unwatch(&file_path)?;
            watched_set.paths.remove(&file_path);
        }

        Ok(())
    }

    pub async fn start_watcher(&self) {
        let mut watched_set = self.watched_set.lock().await;

        watched_set.watcher.start();
    }

    // no need to stop watcher or clear the store because it will be stopped (when dropped) on the store drop
}

fn process_event(event: &Event) -> Result<Option<FileData>> {
    debug!("Processing file watcher event: {event:?}");

    let EventKind::Modify(ModifyKind::Data(_)) = event.kind else {
        debug!("Got not a file data modify event, skipping: {event:?}");
        return Ok(None);
    };

    let path = event.paths.first().cloned().ok_or(anyhow!(
        "event did not provide the path of the modified file"
    ))?;

    let body = fs::read_to_string(&path).with_context(|| "Could not read the modified file")?;

    Ok(Some(FileData::new(path, body)?))
}

fn log_sync_error(err: SyncError) {
    match err {
        SyncError::InvalidJson(source) => {
            error!("Invalid JSON in config file: {}", source);
        }
        SyncError::InvalidConfig(message) => {
            error!("Invalid config file structure: {}", message);
        }
        SyncError::Github(source) => {
            error!("Could not sync saved file due to Github error: {}", source);
        }
        SyncError::Internal(source) => {
            error!(
                "Could not sync saved file due to an internal error: {:?}",
                source
            );
        }
        SyncError::UnhandledInternal(message) => {
            error!("{message}");
        }
    }
}
