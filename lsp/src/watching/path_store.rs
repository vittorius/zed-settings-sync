use std::{path::PathBuf, pin::Pin, sync::Arc};

use anyhow::Result;
use anyhow::{Context, anyhow};
use common::sync::{Client, LocalFileData};
use mockall_double::double;
use notify::{Event, EventKind, event::ModifyKind};
use tokio::fs;
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
                match process_event(&event).await {
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

async fn process_event(event: &Event) -> Result<Option<LocalFileData>> {
    debug!("Processing file watcher event: {event:?}");

    let EventKind::Modify(ModifyKind::Data(_)) = event.kind else {
        debug!("Got not a file data modify event, skipping: {event:?}");
        return Ok(None);
    };

    let path = event.paths.first().cloned().ok_or(anyhow!(
        "event did not provide the path of the modified file"
    ))?;

    let body = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Could not read the modified file: {}", path.display()))?;

    Ok(Some(LocalFileData::new(path, body)?))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use assert_fs::{TempDir, prelude::*};
    use common::sync::MockGithubClient;
    use mockall::{Sequence, predicate};
    use paste::paste;

    use super::*;
    use crate::watching::MockWatchedSet;

    #[tokio::test]
    async fn test_successful_creation() {
        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(|_| Ok(MockWatchedSet::default()));

        assert!(PathStore::new(Arc::new(MockGithubClient::default())).is_ok());
    }

    #[tokio::test]
    async fn test_unsuccessful_creation_when_watched_set_creation_failed() {
        let ctx = MockWatchedSet::new_context();
        ctx.expect()
            .returning(|_| Err(anyhow!("Failed to create watched set")));

        assert!(PathStore::new(Arc::new(MockGithubClient::default())).is_err());
    }

    macro_rules! setup_watched_set_mock {
        ($method:ident, $path:expr) => {
            paste! {
                let ctx = MockWatchedSet::new_context();
                ctx.expect().returning(move |_| {
                    let mut seq = Sequence::new();
                    let mut mock_watched_set = MockWatchedSet::default();
                    mock_watched_set
                        .expect_start_watcher()
                        .in_sequence(&mut seq)
                        .returning(|| ());
                    mock_watched_set
                        .[<expect_ $method>]()
                        .with(predicate::eq($path))
                        .in_sequence(&mut seq)
                        .returning(|_| Ok(()));

                    Ok(mock_watched_set)
                });
            }
        };
        ($method:ident, $path:expr, $err_msg:expr) => {
            paste! {
                let ctx = MockWatchedSet::new_context();
                ctx.expect().returning(move |_| {
                    let mut seq = Sequence::new();
                    let mut mock_watched_set = MockWatchedSet::default();
                    mock_watched_set
                        .expect_start_watcher()
                        .in_sequence(&mut seq)
                        .returning(|| ());
                    mock_watched_set
                        .[<expect_ $method>]()
                        .with(predicate::eq($path))
                        .in_sequence(&mut seq)
                        .returning(|_| Err(anyhow!($err_msg)));

                    Ok(mock_watched_set)
                });
            }
        };
    }

    #[tokio::test]
    async fn test_successful_watch_path() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(watch, path.clone());

        let mut store = PathStore::new(Arc::new(MockGithubClient::default()))?;
        store.start_watcher();
        store.watch(path_clone)?;

        Ok(())
    }

    #[tokio::test]
    async fn test_unsuccessful_watch_path() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(watch, path.clone(), "Path already being watched");

        let mut store = PathStore::new(Arc::new(MockGithubClient::default()))?;
        store.start_watcher();

        assert_eq!(
            store.watch(path_clone).unwrap_err().to_string(),
            "Path already being watched"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_successful_unwatch_path() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(unwatch, path.clone());

        let mut store = PathStore::new(Arc::new(MockGithubClient::default()))?;
        store.start_watcher();
        store.unwatch(&path_clone)?;

        Ok(())
    }

    #[tokio::test]
    async fn test_unsuccessful_unwatch_path() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(unwatch, path.clone(), "Path was not watched");

        let mut store = PathStore::new(Arc::new(MockGithubClient::default()))?;
        store.start_watcher();

        assert_eq!(
            store.unwatch(&path_clone).unwrap_err().to_string(),
            "Path was not watched"
        );

        Ok(())
    }

    /*

    Integration tests, here or just for WatchedSet (TODO)

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
