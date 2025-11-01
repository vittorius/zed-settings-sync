use notify::Watcher;
use std::{path::Path, pin::Pin, sync::Mutex};

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode};
use tokio::{sync::mpsc::channel, task};
use tracing::error;

pub type EventHandler =
    Box<dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static>;

#[derive(Debug)]
pub struct PathWatcher {
    // behind mutex to control watch/unwatch operations
    watcher: Mutex<RecommendedWatcher>,
}

impl PathWatcher {
    pub fn new(event_handler: EventHandler) -> Result<Self> {
        let (tx, mut rx) = channel(1);
        let handle = tokio::runtime::Handle::current();

        let watcher = RecommendedWatcher::new(
            move |res| {
                // called from a thread that is not controlled by tokio,
                // so need to enter tokio async context explicitly
                handle.block_on(async {
                    tx.send(res).await.unwrap();
                });
            },
            Config::default(),
        )?;

        task::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => (event_handler)(event).await,
                    Err(e) => error!("Path watcher error: {}", e),
                }
            }
        });

        Ok(Self {
            watcher: Mutex::new(watcher),
        })
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher
            .lock()
            .expect("Path watcher mutex is poisoned")
            .watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(())
    }

    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher
            .lock()
            .expect("Path watcher mutex is poisoned")
            .unwatch(path.as_ref())?;

        Ok(())
    }
}
