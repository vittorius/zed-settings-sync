use std::path::Path;
use std::{path::PathBuf, pin::Pin, sync::Arc};

use anyhow::Result;
use anyhow::{Context, anyhow};
use common::sync::{Client as SyncClient, LocalFileData};
use mockall_double::double;
use notify::{Event, EventKind, event::ModifyKind};
use tokio::fs;
#[cfg(not(test))]
use tower_lsp::Client as LspClient;
use tower_lsp::lsp_types::MessageType;
use tracing::{debug, error};

#[cfg(test)]
use crate::mocks::MockLspClient as LspClient;
#[double]
use crate::watching::WatchedSet;

#[derive(Debug)]
pub struct PathStore {
    watched_set: WatchedSet,
}

#[cfg_attr(test, mockall::automock)]
impl PathStore {
    pub fn new(sync_client: Arc<dyn SyncClient>, lsp_client: Arc<LspClient>) -> Result<Self> {
        let event_handler = Box::new(move |event| {
            let sync_client_clone = Arc::clone(&sync_client);
            let lsp_client_clone = Arc::clone(&lsp_client);

            Box::pin(async move {
                match process_event(&event).await {
                    Ok(data) => {
                        let Some(data) = data else {
                            return;
                        };

                        match sync_client_clone.sync_file(data).await {
                            Ok(()) => {
                                lsp_client_clone
                                    .show_message(
                                        MessageType::INFO,
                                        "Successfully synced".to_owned(),
                                    )
                                    .await;
                            }
                            Err(err) => {
                                error!("Could not sync file: {err}");
                                lsp_client_clone
                                    .show_message(MessageType::ERROR, err.to_string())
                                    .await;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Could not process file event: {err}");
                        lsp_client_clone
                            .show_message(
                                MessageType::ERROR,
                                "File watcher internal error, check LSP server logs".to_owned(),
                            )
                            .await;
                    }
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

    pub fn unwatch(&mut self, file_path: &Path) -> anyhow::Result<()> {
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

    use assert_fs::{NamedTempFile, TempDir, prelude::*};
    use common::sync::{Error, FileError, MockGithubClient};
    use mockall::predicate;
    use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, RemoveKind};
    use paste::paste;
    use tokio::runtime::Runtime;

    use super::*;
    use crate::{
        mocks::MockLspClient,
        watching::{EventHandler, MockWatchedSet},
    };

    #[test]
    fn test_creation_success() {
        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(|_| Ok(MockWatchedSet::default()));

        assert!(
            PathStore::new(
                Arc::new(MockGithubClient::default()),
                Arc::new(MockLspClient::default())
            )
            .is_ok()
        );
    }

    #[test]
    fn test_creation_failure_when_watched_set_creation_failed() {
        let ctx = MockWatchedSet::new_context();
        ctx.expect()
            .returning(|_| Err(anyhow!("Failed to create watched set")));

        assert!(
            PathStore::new(
                Arc::new(MockGithubClient::default()),
                Arc::new(MockLspClient::default())
            )
            .is_err()
        );
    }

    macro_rules! setup_watched_set_mock {
        ($method:ident, $path:expr) => {
            paste! {
                let ctx = MockWatchedSet::new_context();
                ctx.expect().returning(move |_| {
                    let mut seq = mockall::Sequence::new();
                    let mut mock_watched_set = MockWatchedSet::default();
                    mock_watched_set
                        .expect_start_watcher()
                        .in_sequence(&mut seq)
                        .returning(|| ());
                    mock_watched_set
                        .[<expect_ $method>]()
                        .with(mockall::predicate::eq($path))
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
                    let mut seq = mockall::Sequence::new();
                    let mut mock_watched_set = MockWatchedSet::default();
                    mock_watched_set
                        .expect_start_watcher()
                        .in_sequence(&mut seq)
                        .returning(|| ());
                    mock_watched_set
                        .[<expect_ $method>]()
                        .with(mockall::predicate::eq($path))
                        .in_sequence(&mut seq)
                        .returning(|_| Err(anyhow!($err_msg)));

                    Ok(mock_watched_set)
                });
            }
        };
    }

    #[test]
    fn test_watch_path_success() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(watch, path.clone());

        let mut store = PathStore::new(
            Arc::new(MockGithubClient::default()),
            Arc::new(MockLspClient::default()),
        )?;
        store.start_watcher();
        store.watch(path_clone)?;

        Ok(())
    }

    #[test]
    fn test_watch_path_failure() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(watch, path.clone(), "Path already being watched");

        let mut store = PathStore::new(
            Arc::new(MockGithubClient::default()),
            Arc::new(MockLspClient::default()),
        )?;
        store.start_watcher();

        assert_eq!(
            store.watch(path_clone).unwrap_err().to_string(),
            "Path already being watched"
        );

        Ok(())
    }

    #[test]
    fn test_unwatch_path_success() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(unwatch, path.clone());

        let mut store = PathStore::new(
            Arc::new(MockGithubClient::default()),
            Arc::new(MockLspClient::default()),
        )?;
        store.start_watcher();
        store.unwatch(&path_clone)?;

        Ok(())
    }

    #[test]
    fn test_unwatch_path_failure() -> Result<()> {
        let dir = TempDir::new()?;
        dir.child("foobar").touch()?;
        let path = dir.path().to_path_buf();
        let path_clone = path.clone();

        setup_watched_set_mock!(unwatch, path.clone(), "Path was not watched");

        let mut store = PathStore::new(
            Arc::new(MockGithubClient::default()),
            Arc::new(MockLspClient::default()),
        )?;
        store.start_watcher();

        assert_eq!(
            store.unwatch(&path_clone).unwrap_err().to_string(),
            "Path was not watched"
        );

        Ok(())
    }

    #[test]
    fn test_non_modify_event_handling() -> Result<()> {
        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(move |event_handler: EventHandler| {
            let non_modify_events = [
                Event::new(EventKind::Access(AccessKind::Read)),
                Event::new(EventKind::Access(AccessKind::Open(AccessMode::Any))),
                Event::new(EventKind::Create(CreateKind::File)),
                Event::new(EventKind::Create(CreateKind::Folder)),
                Event::new(EventKind::Remove(RemoveKind::File)),
                Event::new(EventKind::Remove(RemoveKind::Folder)),
            ];

            let rt = Runtime::new()?;
            rt.block_on(async {
                for event in non_modify_events {
                    event_handler(event).await;
                }
            });

            let mock_watched_set = MockWatchedSet::default();
            Ok(mock_watched_set)
        });

        let mut mock_sync_client = MockGithubClient::default();
        mock_sync_client.expect_sync_file().never();

        PathStore::new(
            Arc::new(mock_sync_client),
            Arc::new(MockLspClient::default()),
        )?;

        Ok(())
    }

    #[test]
    fn test_modify_event_without_modified_path_notification() -> Result<()> {
        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(move |event_handler: EventHandler| {
            let event = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any)));

            let rt = Runtime::new()?;
            rt.block_on(async {
                event_handler(event).await;
            });

            let mock_watched_set = MockWatchedSet::default();
            Ok(mock_watched_set)
        });

        let mut mock_sync_client = MockGithubClient::default();
        mock_sync_client.expect_sync_file().never();

        let mut mock_lsp_client = MockLspClient::default();
        mock_lsp_client
            .expect_show_message()
            .with(
                predicate::eq(MessageType::ERROR),
                predicate::eq("File watcher internal error, check LSP server logs".to_owned()),
            )
            .return_once(|_msg_type, _msg| Box::pin(async {}));

        PathStore::new(Arc::new(mock_sync_client), Arc::new(mock_lsp_client))?;

        Ok(())
    }

    #[test]
    fn test_modify_event_with_file_read_error_notification() -> Result<()> {
        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(move |event_handler: EventHandler| {
            let mut event = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any)));
            event = event.add_path(PathBuf::from("non-existent-file"));

            let rt = Runtime::new()?;
            rt.block_on(async {
                event_handler(event).await;
            });

            let mock_watched_set = MockWatchedSet::default();
            Ok(mock_watched_set)
        });

        let mut mock_sync_client = MockGithubClient::default();
        mock_sync_client.expect_sync_file().never();

        let mut mock_lsp_client = MockLspClient::default();
        mock_lsp_client
            .expect_show_message()
            .with(
                predicate::eq(MessageType::ERROR),
                predicate::eq("File watcher internal error, check LSP server logs".to_owned()),
            )
            .return_once(|_msg_type, _msg| Box::pin(async {}));

        PathStore::new(Arc::new(mock_sync_client), Arc::new(mock_lsp_client))?;

        Ok(())
    }

    #[test]
    fn test_modify_event_handling_with_sync_success() -> Result<()> {
        let temp_file = NamedTempFile::new("settings.json")?;
        temp_file.write_str(r#"{ "hello": "kitty" }"#)?;
        let temp_file_path = temp_file.path().to_path_buf();

        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(move |event_handler: EventHandler| {
            let mut event = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)));
            event = event.add_path(temp_file.path().to_path_buf());

            let rt = Runtime::new()?;
            rt.block_on(async {
                event_handler(event).await;
            });

            let mock_watched_set = MockWatchedSet::default();
            Ok(mock_watched_set)
        });

        let file_data = LocalFileData::new(temp_file_path, r#"{ "hello": "kitty" }"#.into())?;

        let mut mock_sync_client = MockGithubClient::default();
        mock_sync_client
            .expect_sync_file()
            .with(predicate::eq(file_data))
            .return_once(|_| Ok(()));

        let mut mock_lsp_client = MockLspClient::default();
        mock_lsp_client
            .expect_show_message()
            .with(
                predicate::eq(MessageType::INFO),
                predicate::eq("Successfully synced".to_owned()),
            )
            .return_once(|_msg_type, _msg| Box::pin(async {}));

        PathStore::new(Arc::new(mock_sync_client), Arc::new(mock_lsp_client))?;

        Ok(())
    }

    #[test]
    fn test_modify_event_handling_with_sync_failure() -> Result<()> {
        let temp_file = NamedTempFile::new("settings.json")?;
        temp_file.write_str(r#"{ "hello": "kitty" }"#)?;
        let temp_file_path = temp_file.path().to_path_buf();

        let ctx = MockWatchedSet::new_context();
        ctx.expect().returning(move |event_handler: EventHandler| {
            let mut event = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)));
            event = event.add_path(temp_file.path().to_path_buf());

            let rt = Runtime::new()?;
            rt.block_on(async {
                event_handler(event).await;
            });

            let mock_watched_set = MockWatchedSet::default();
            Ok(mock_watched_set)
        });

        let file_data = LocalFileData::new(temp_file_path, r#"{ "hello": "kitty" }"#.into())?;

        let mut mock_sync_client = MockGithubClient::default();
        mock_sync_client
            .expect_sync_file()
            .with(predicate::eq(file_data))
            .return_once(|_| {
                Err(FileError::from_error(
                    "settings.json",
                    Error::UnhandledInternal("Sync error".into()),
                ))
            });

        let mut mock_lsp_client = MockLspClient::default();
        mock_lsp_client
            .expect_show_message()
            .with(
                predicate::eq(MessageType::ERROR),
                predicate::eq("Error syncing file settings.json: Unhandled internal error from underlying client library: Sync error".to_owned()),
            )
            .return_once(|_msg_type, _msg| Box::pin(async {}));

        PathStore::new(Arc::new(mock_sync_client), Arc::new(mock_lsp_client))?;

        Ok(())
    }
}
