## Goals

- No settings edit for any settings file must be lost
- Add edits should be applied synced to the external storage (Gist) in the same order they were applied to the file
- No excessive synchronizations: as few requests to the external storage as possible

## Architecture v0.2

- Upon its initialization, the LSP server:
  - checks if there is no "global" watcher process started (the one that watches ~/.config/zed/**)
    - if none, it starts it (paths are taken from Zed's "paths" crate)
    - the "global" watcher process should be created in such a way, that it will be inherited by Zed process after this LSP process is shut down or restarted, so, it will be stopped when Zed is quit
  - gets the current workspace path
  - creates `.zed` directory in that workspace if it doesn't exist (paths are taken from Zed's "paths" crate) 
  - starts the local watcher process to watch over this workspace's Zed config directory
  
- Upon its de-initialization, the LSP server:
  - stops the local watcher process for this workspace's Zed config directory

## Architecture v0.1 

❌ Obsolete: Zed doesn't emit didSave LSP message when working with global settings files because they are opened in a separate invisible worktree. It means that pure LSP approach won't work.

- Application state is a map of \[file URI -> queue of file updates]
- When a config file (determined by path matching) is saved, this change data is pushed to the queue
- The consumer reads from the opposite side of the queue and saves changes to a Gist
- Maybe, we need just a single queue. But with multiple queues it will be easier to control debouncing. E.g. draining the queue (except for the last piece of change data), when the "file closed" LSP event was received for this file.