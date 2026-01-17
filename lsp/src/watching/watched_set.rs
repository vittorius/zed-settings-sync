use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{Result, bail};
use mockall_double::double;

use crate::watching::EventHandler;
#[double]
use crate::watching::PathWatcher;

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

    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::{path::PathBuf, sync::OnceLock};

    use anyhow::Result;
    use notify::{Event, EventKind};
    use tokio::runtime::Runtime;

    use crate::watching::{EventHandler, MockPathWatcher, WatchedSet};

    fn new_watched_set() -> Result<WatchedSet> {
        WatchedSet::new(Box::new(|_ev: Event| Box::pin(async {})))
    }

    #[test]
    fn test_new_successful() -> Result<()> {
        static EVENT_HANDLER_CALLED: OnceLock<bool> = OnceLock::new();

        let event_handler: EventHandler = Box::new(|_| {
            Box::pin(async {
                EVENT_HANDLER_CALLED
                    .set(true)
                    .expect("Flag was already set");
            })
        });

        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|event_handler| {
            let rt = Runtime::new()?;
            rt.block_on(async {
                event_handler(Event::new(EventKind::Any)).await;
            });
            Ok(MockPathWatcher::default())
        });

        let set = WatchedSet::new(event_handler)?;
        assert!(set.paths.is_empty());
        assert!(EVENT_HANDLER_CALLED.get().expect("Flag was not set")); // testing that WatchedSet passes the event handler to PathWatcher

        Ok(())
    }

    #[test]
    fn test_start_watcher_successful() -> Result<()> {
        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|_| {
            let mut mock_path_watcher = MockPathWatcher::default();
            mock_path_watcher.expect_start().return_once(|| ());
            Ok(mock_path_watcher)
        });

        let mut set = new_watched_set()?;
        set.start_watcher();

        Ok(())
    }

    #[test]
    fn test_watch_successful() -> Result<()> {
        let path = PathBuf::from("/hello/there");
        let path_clone = path.clone();

        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|_| {
            let mut mock_path_watcher = MockPathWatcher::default();
            mock_path_watcher.expect_watch().return_once(move |path| {
                assert_eq!(path, path_clone);
                Ok(())
            });

            Ok(mock_path_watcher)
        });

        let mut set = new_watched_set()?;
        set.watch(path.clone())?;
        assert!(set.paths.contains(&path));

        Ok(())
    }

    #[test]
    fn test_watch_failure_if_already_watched() -> Result<()> {
        let path = PathBuf::from("/hello/there");
        let path_clone = path.clone();

        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|_| {
            let mut mock_path_watcher = MockPathWatcher::default();
            mock_path_watcher.expect_watch().return_once(move |path| {
                assert_eq!(path, path_clone);
                Ok(())
            });

            Ok(mock_path_watcher)
        });

        let mut set = new_watched_set()?;
        set.watch(path.clone())?;
        assert_eq!(
            set.watch(path.clone()).unwrap_err().to_string(),
            "Path is already being watched: /hello/there"
        );

        Ok(())
    }

    #[test]
    fn test_unwatch_successful() -> Result<()> {
        let path = PathBuf::from("/hello/there");
        let path_clone_to_watch = path.clone();
        let path_clone_to_unwatch = path.clone();

        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|_| {
            let mut mock_path_watcher = MockPathWatcher::default();
            mock_path_watcher.expect_watch().return_once(move |path| {
                assert_eq!(path, path_clone_to_watch);
                Ok(())
            });
            mock_path_watcher.expect_unwatch().return_once(move |path| {
                assert_eq!(path, path_clone_to_unwatch);
                Ok(())
            });

            Ok(mock_path_watcher)
        });

        let mut set = new_watched_set()?;
        set.watch(path.clone())?;
        set.unwatch(&path)?;
        assert!(!set.paths.contains(&path));

        Ok(())
    }

    #[test]
    fn test_unwatch_failure_if_not_watched() -> Result<()> {
        let path = PathBuf::from("/hello/there");
        let path_clone_to_unwatch = path.clone();

        let ctx = MockPathWatcher::new_context();
        ctx.expect().return_once(|_| {
            let mut mock_path_watcher = MockPathWatcher::default();
            mock_path_watcher.expect_unwatch().return_once(move |path| {
                assert_eq!(path, path_clone_to_unwatch);
                Ok(())
            });

            Ok(mock_path_watcher)
        });

        let mut set = new_watched_set()?;
        assert_eq!(
            set.unwatch(&path).unwrap_err().to_string(),
            "Path is not being watched, failed to unwatch: /hello/there"
        );

        Ok(())
    }
}
