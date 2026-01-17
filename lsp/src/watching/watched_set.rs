use std::{collections::HashSet, path::PathBuf, sync::Mutex};

use anyhow::{Result, bail};

use crate::watching::{EventHandler, PathWatcher};

#[derive(Debug)]
pub struct WatchedSet {
    paths: HashSet<PathBuf>,
    watcher: PathWatcher,
    mx: Mutex<()>,
}

#[cfg_attr(test, mockall::automock)]
impl WatchedSet {
    pub fn new(event_handler: EventHandler) -> Result<Self> {
        let watcher = PathWatcher::new(event_handler)?;

        Ok(Self {
            paths: HashSet::new(),
            watcher,
            mx: Mutex::new(()),
        })
    }

    pub fn start_watcher(&mut self) {
        #[allow(clippy::expect_used)]
        let _lock = self.mx.lock().expect("Watched set mutex is poisoned");

        self.watcher.start();
    }

    pub fn watch(&mut self, path: PathBuf) -> Result<()> {
        #[allow(clippy::expect_used)]
        let _lock = self.mx.lock().expect("Watched set mutex is poisoned");

        if self.paths.contains(&path) {
            bail!("Path is already being watched: {}", path.display());
        }

        self.watcher.watch(&path)?;
        self.paths.insert(path);

        Ok(())
    }

    pub fn unwatch(&mut self, path: &PathBuf) -> Result<()> {
        #[allow(clippy::expect_used)]
        let _lock = self.mx.lock().expect("Watched set mutex is poisoned");

        if !self.paths.contains(path) {
            bail!(
                "Path is not being watched, failed to unwatch: {}",
                path.display()
            );
        }

        self.watcher.unwatch(path)?;
        self.paths.remove(path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
