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

    pub fn watch<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.watcher
            .lock()
            .map_err(|_| anyhow!("Path watcher mutex is poisoned"))?
            .watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(())
    }

    pub fn unwatch<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.watcher
            .lock()
            .map_err(|_| anyhow!("Path watcher mutex is poisoned"))?
            .unwatch(path.as_ref())?;

        Ok(())
    }
}
