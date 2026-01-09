use std::{fs, path::PathBuf, pin::Pin, sync::Arc};

use anyhow::Result;
use anyhow::{Context, anyhow};
use common::sync::{Client, LocalFileData};
use mockall_double::double;
use notify::{Event, EventKind, event::ModifyKind};
use tracing::{debug, error};

#[double]
use crate::watching::WatchedSet;

#[derive(Debug)]
pub struct PathStore {
    watched_set: WatchedSet,
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
            watched_set: WatchedSet::new(event_handler)?,
        })
    }

    pub fn start_watcher(&mut self) {
        self.watched_set.start_watcher();
    }

    // no need to stop watcher or clear the store because it will be stopped (when dropped) on the store drop

    pub fn watch(&mut self, file_path: PathBuf) -> anyhow::Result<()> {
        self.watched_set.watch(file_path)
    }

    pub fn unwatch(&mut self, file_path: &PathBuf) -> anyhow::Result<()> {
        self.watched_set.unwatch(file_path)
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
