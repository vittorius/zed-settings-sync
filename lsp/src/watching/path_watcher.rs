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
                    Err(e) => error!("Path watcher error: {}", e),
                }
            }
        });
    }

    pub fn watch(&self, path: &Path) -> Result<()> {
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

mod tests {

    /*
    Tests TODO
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
