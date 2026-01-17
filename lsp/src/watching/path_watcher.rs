use std::{path::Path, pin::Pin, sync::Mutex};

use anyhow::{Result, anyhow};
use debug_ignore::DebugIgnore;
use notify::Watcher;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result as NotifyResult};
use tokio::{
    sync::mpsc::{Receiver, channel},
    task,
};
use tracing::error;

pub type EventHandler =
    Box<dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static>;

#[derive(Debug)]
pub struct PathWatcher {
    watcher: Mutex<RecommendedWatcher>, // behind mutex to control watch/unwatch operations
    rx: Option<Receiver<NotifyResult<Event>>>,
    event_handler: Option<DebugIgnore<EventHandler>>, // DebugIgnore because Fn traits can't implement Debug
}

#[cfg_attr(test, mockall::automock)]
impl PathWatcher {
    pub fn new(event_handler: EventHandler) -> Result<Self> {
        let (tx, rx) = channel(1);
        let handle = tokio::runtime::Handle::current();

        let watcher = RecommendedWatcher::new(
            move |res| {
                // called from a thread that is not controlled by tokio,
                // so need to enter tokio async context explicitly
                handle.block_on(async {
                    if tx.send(res).await.is_err() {
                        // TODO: propagate this error to the path store level so it can be displayed to a Zed user
                        error!("Path watcher receiver dropped or closed");
                    }
                });
            },
            Config::default(),
        )?;

        Ok(Self {
            watcher: Mutex::new(watcher),
            rx: Some(rx),
            event_handler: Some(DebugIgnore(event_handler)),
        })
    }

    pub fn start(&mut self) {
        #[allow(clippy::expect_used)]
        let mut rx = self
            .rx
            .take()
            .expect("Path watcher receiver must be initialized");

        #[allow(clippy::expect_used)]
        let event_handler = self
            .event_handler
            .take()
            .expect("Event handler must be initialized");

        task::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => (event_handler)(event).await,
                    Err(e) => {
                        // TODO: propagate this error to the path store level so it can be displayed to a Zed user
                        error!("Path watcher error: {}", e);
                    }
                }
            }
        });
    }

    pub fn watch(&self, path: &Path) -> Result<()> {
        // println!("Watcher is running: {}", self.watcher.lock().unwrap())
        self.watcher
            .lock()
            .map_err(|_| anyhow!("Path watcher mutex is poisoned"))?
            .watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(())
    }

    pub fn unwatch(&self, path: &Path) -> Result<()> {
        self.watcher
            .lock()
            .map_err(|_| anyhow!("Path watcher mutex is poisoned"))?
            .unwatch(path.as_ref())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };

    use anyhow::Result;
    use assert_fs::{TempDir, prelude::*};

    use crate::watching::{EventHandler, PathWatcher};

    macro_rules! init_event_handler {
        ($var:ident) => {
            static EVENT_HANDLER_CALLED: AtomicBool = AtomicBool::new(false);
            let $var: EventHandler = Box::new(|_| {
                Box::pin(async {
                    set_event_handler_called!();
                })
            });
        };
    }

    macro_rules! clear_event_handler_called {
        () => {
            EVENT_HANDLER_CALLED.store(false, Ordering::Relaxed);
        };
    }

    macro_rules! set_event_handler_called {
        () => {
            EVENT_HANDLER_CALLED.store(true, Ordering::Relaxed);
        };
    }

    async fn assert_event_handler_called_with(
        event_handler_called: &AtomicBool,
        value: bool,
    ) -> bool {
        let mut duration = 100;
        while duration <= 2000 {
            if event_handler_called.load(Ordering::Relaxed) == value {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(duration)).await;
            duration *= 2;
        }

        false
    }

    macro_rules! assert_event_handler_called {
        () => {
            assert_event_handler_called_with(&EVENT_HANDLER_CALLED, true).await;
        };
    }

    macro_rules! assert_event_handler_not_called {
        () => {
            assert_event_handler_called_with(&EVENT_HANDLER_CALLED, false).await;
        };
    }

    #[tokio::test]
    async fn test_file_event_inside_watched_path_is_caught() -> Result<()> {
        init_event_handler!(event_handler);

        let mut path_watcher = PathWatcher::new(event_handler)?;
        let dir_watched = TempDir::new()?;

        path_watcher.start();
        path_watcher.watch(dir_watched.path())?;
        dir_watched.child("file.txt").write_str("Hello, world!\n")?;

        assert_event_handler_called!();

        Ok(())
    }

    #[tokio::test]
    async fn test_file_event_outside_of_watched_path_is_ignored() -> Result<()> {
        init_event_handler!(event_handler);

        let mut path_watcher = PathWatcher::new(event_handler)?;

        let root_dir = TempDir::new()?;
        let child_dir_watched = root_dir.child("with_changes");
        child_dir_watched.create_dir_all()?;
        let child_dir_ignored = root_dir.child("ignored");
        child_dir_ignored.create_dir_all()?;

        path_watcher.start();
        path_watcher.watch(child_dir_watched.path())?;
        child_dir_ignored
            .child("file.txt")
            .write_str("Hello, world!")?;

        assert_event_handler_not_called!();

        Ok(())
    }

    #[tokio::test]
    async fn test_unwatch_successful() -> Result<()> {
        init_event_handler!(event_handler);

        let mut path_watcher = PathWatcher::new(event_handler)?;
        let dir_watched = TempDir::new()?;

        path_watcher.start();
        path_watcher.watch(dir_watched.path())?;
        dir_watched.child("file.txt").write_str("Hello, world!\n")?;

        assert_event_handler_called!();

        clear_event_handler_called!();

        path_watcher.unwatch(dir_watched.path())?;
        dir_watched
            .child("another_file.txt")
            .write_str("Hello, kitty!\n")?;

        assert_event_handler_not_called!();

        Ok(())
    }
}
