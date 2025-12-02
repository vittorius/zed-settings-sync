# Goals

- No settings edit for any settings file must be lost
- Add edits should be applied synced to the external storage (Gist) in the same order they were applied to the file
- No excessive synchronizations: as few requests to the external storage as possible

## Architecture v0.0.4

- Watch for user settings files' changes only (not project ones)
- On didOpen, if a file is a JSON file and in global config dir, add it to the watch list
- On didClose, remove a file from the watch list if the it's there
- ~~Update configuration handler (probably, via the corresponding LSP event), so that the LSP server catches up the gist id or auth token change in the app settings~~. It's enough to use `initialization_options` to pass the auth token and gist id to the LSP server - it will be restarted on their change.
- CLI binary crate to download user settings files from the Gist. `zed-settings-sync load –-auth-token <GitHub auth token> –-gist <Gist ID>`. There will be a prompt to either overwrite, backup, or ignore on every file (settings.json, keymap.json, etc.) that will be attempted to upload.
- A watcher Tokio "thread" should listen to the create/change file events and invoke the Github client to save their contents to the cloud.
- A Github token used to authenticate with the Github API is masked on the fly during sync if syncing the `settings.json` file.

## Architecture v0.0.3

❌ Obsolete: watching local settings files is impractical since they can be just checked into the VCS by a developer. Also, only didOpen/didClose events are available to us.

- When an LSP server started place watchers on global settings file and global keymap file
- On didOpen, if a file is in either global config dir or local config dir, add it to the watch list
  - Or, add the entire global/local config dir to the watch list, recursively
  - TODO: in both cases, check how watcher behaves in case of full deletion of the local config dir
    - If a config dir is deleted, remove it from the watch list
- On didClose, remove a file from the watch list if the it's there
- A watcher Tokio "thread" should listen to the create/change file events and save their contents to the cloud

Details:

- AppState contains a HashMap of watched directories: PathBuf -> FsEventWatcher
  - This should be a separate watched store type
- Two types of watchers:
- GlobalConfigWatcher (accepts any file changes under its watched dir)
- ProjectDirConfigWatcher (watched the entire project folder and filters in only files that match the local config path pattern)
- FileConfigWatcher (watches a single config file by its full path)
- How's the new path is added to watched dirs (addition happens only if a watched dir path is not yet added to the store):
  - launch -> add global config dir to watched dirs using the GlobalConfigWatcher
  - launch and workspace is present -> add workspace root to watched dirs using ProjectDirConfigWatcher
  - didOpen
    - if matches global config pattern, add global config dir using the GlobalConfigWatcher
    - else if matches local settings path (ends with .zed/\*.json), add the file. **Possible issue** here: duplicate events logging from both project dir and file (under project dir) watchers.
- How's the new path is removed from watched dirs:
  - didClose: ?

Test cases:

- open a workspace with a single project dir
  - open settings file for editing
  - open keymap file for editing
  - open project settings for editing
  - open project tasks for editing
- open a workspace with multiple project dirs
  - open settings file for editing
  - open keymap file for editing
  - open project settings for editing (from various project dirs)
  - open project tasks for editing (from various project dirs)
- open Zed without any workspace open and open settings file for editing
- open Zed without any workspace open and open keymap file for editing
- delete local config dir (./.zed) **DON'T FORGET TO BACK IT UP**
- delete global config dir (~/.config/zed) **DON'T FORGET TO BACK IT UP**

PoC #1: let's implement the logging of all events, remove logs garbage and find out all possible cases
Outcome #1: there is no point in relying on workspace folders or root_uri because they don't correlate with actual files open. It will be easier to rely on concrete files open/close and deduce watched paths from them
Outcome #2: it worked.
Verdict: using this approach further on.

## Architecture v0.0.2

❌ Obsolete: having 2 processes and coordinating them is too complex, let's place the file watcher Tokio "thread" within the LSP server. The "app state" will hold all paths to currently watched dirs or files.

- Upon its initialization, the LSP server:
  - checks if there is no "global" watcher process started (the one that watches ~/.config/zed/\*\*)
    - if none, it starts it (paths are taken from Zed's "paths" crate)
    - the "global" watcher process should be created in such a way, that it will be inherited by Zed process after this LSP process is shut down or restarted, so, it will be stopped when Zed is quit
  - gets the current workspace path
  - creates `.zed` directory in that workspace if it doesn't exist (paths are taken from Zed's "paths" crate)
  - starts the local watcher process to watch over this workspace's Zed config directory

- Upon its de-initialization, the LSP server:
  - stops the local watcher process for this workspace's Zed config directory

## Architecture v0.0.1

❌ Obsolete: Zed doesn't emit didSave LSP message when working with global settings files because they are opened in a separate invisible worktree. It means that pure LSP approach won't work.

- Application state is a map of \[file URI -> queue of file updates]
- When a config file (determined by path matching) is saved, this change data is pushed to the queue
- The consumer reads from the opposite side of the queue and saves changes to a Gist
- Maybe, we need just a single queue. But with multiple queues it will be easier to control debouncing. E.g. draining the queue (except for the last piece of change data), when the "file closed" LSP event was received for this file.
