use std::{collections::HashSet, fs, path::PathBuf, pin::Pin, sync::Arc};

use anyhow::Result;
use anyhow::{Context, anyhow, bail};
use common::sync::{Client, LocalFileData};
use notify::{Event, EventKind, event::ModifyKind};
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::watching::{EventHandler, PathWatcher};

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
pub struct PathStore {
    // behind mutex to control the simultaneous change of paths set and path watcher
    watched_set: Mutex<WatchedSet>,
}

impl PathStore {
    pub fn new(client: Arc<dyn Client>) -> Result<Self> {
        let event_handler = Box::new(move |event| {
            let client_clone = Arc::clone(&client);
            Box::pin(async move {
                match process_event(&event) {
                    Ok(data) => {
                        let Some(data) = data else {
                            return;
                        };

                        if let Err(err) = client_clone.sync_file(data).await {
                            error!("{}", err);
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

    pub async fn start_watcher(&self) {
        let mut watched_set = self.watched_set.lock().await;

        watched_set.watcher.start();
    }

    // no need to stop watcher or clear the store because it will be stopped (when dropped) on the store drop

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
}

fn process_event(event: &Event) -> Result<Option<LocalFileData>> {
    debug!("Processing file watcher event: {event:?}");

    let EventKind::Modify(ModifyKind::Data(_)) = event.kind else {
        debug!("Got not a file data modify event, skipping: {event:?}");
        return Ok(None);
    };

    let path = event.paths.first().cloned().ok_or(anyhow!(
        "event did not provide the path of the modified file"
    ))?;

    let body = fs::read_to_string(&path).with_context(|| "Could not read the modified file")?;

    Ok(Some(LocalFileData::new(path, body)?))
}

#[cfg(test)]
mod tests {
    use common::sync::MockGithubClient;

    use super::*;

    #[tokio::test]
    async fn test_successful_creation() {
        assert!(PathStore::new(Arc::new(MockGithubClient::default())).is_ok());
    }

    /*
    - new
      - test successful creation
        - create a store with the MockGithubClient passed
      - test unsuccessful creation is watched set creation failed

    - start watcher
      - test successful start watcher
        - watcher started (mock for PathWatcher)

    - watch
      - test successful watch new path
        - new path passed to path watcher for watch (mock for PathWatcher)
        - new path added to watched set (mock for WatchedSet)
      - test failure watch path already watched

    - unwatch
        - test successful unwatch
        - test failure unwatch path not watched
          - path passed to path watcher for unwatch (mock for PathWatcher)
          - path removed from watched set (mock for WatchedSet)

    - events handling
      - test create file does not trigger event handler
        - create a store with the MockGithubClient passed
        - start watcher
        - add a new path to watch (assert_fs::TempDir), maybe with an already existing file
        - create a new file in that dir
        - ensure event was not triggered (MockGithubClient)
      - test delete file does not trigger event handler
        - create a store with the MockGithubClient passed
        - start watcher
        - add a new path to watch (assert_fs::TempDir), with an already existing file
        - delete the file
        - ensure event was not triggered (MockGithubClient)
      - test modify file data triggers event handler
        - create a store with the MockGithubClient passed
        - start watcher
        - add a new path to watch (assert_fs::TempDir), with an already existing file
        - modify the file data
        - ensure event was triggered (MockGithubClient)
      - test modify file data outside of watched paths does not trigger event handler
        - create a store with the MockGithubClient passed
        - start watcher
        - add a new path to watch (assert_fs::TempDir)
        - create another assert_fs::TempDir with an existing file
        - modify that file data
        - ensure event was not triggered (MockGithubClient)
        */
}
