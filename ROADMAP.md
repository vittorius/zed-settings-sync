# Roadmap

## Development

### Overall

- [x] Test the release is working
- [x] Test the extension installation and downloading the released LSP binary from Github is working
- [x] Write a test a script to locally build the LSP binary and copy it to the Zed's folder for the extension (see Zed Discord Presence LSP server info for the full path, also check out the Zed sources, namely paths.rs)
- [x] Forbid the usage of unwrap and expect for Option, Result
- [x] Use serde_json from zed_extension_api, not directly
- [x] To support multiple sync providers, implement a Client trait and use it for all sync operations. Move it to the "common" crate.
- [x] **Add unit tests for all important business logic types**
- [x] Prepare README for the initial public release (see Documentation)

## LSP server

- [x] Report a sync error in a visible way (crash the server? know how to report an LSP error inside Zed?)
- [x] To support multiple sync providers, implement a SyncClient trait and use it for all sync operations. Move it to the "common" crate.
- [ ] Manually save a settings file on its open (before adding to the watch list) to handle the case when the LSP server is restarted after the initialization_options are changes in settings.json file.
- [ ] Print the LSP Rust package version in the logs upon initialization (and add it to the Bug issue template on Github)
- [ ] Test all error-returning code paths and ensure that all error conditions are either properly logged and/or reported back to Zed in form or a JSON-RPC error response
- [ ] Ensure that restarting or shutting down the LSP server doesn't prevent the last coming updates from getting synced; otherwise, mitigate that
- [ ] Backup installed themes to the gist automatically
- [ ] Create a true integration test when a server is spawned, it's fed with JSON-RPC messages, and Github API URL is mocked via env var to point to a local mock server as another process
- [ ] After implementing naive changes persistence (sync files as FS events come), seek the ways to improve it (e.g. queuing events)
      – [ ] (experimental) Rewrite the LSP server to use structured async concurrency, thread-per-code async runtime, and get rid of Arc's around every data structure
- [ ] Use type-state pattern to guarantee that one cannot watch/unwatch a path using non-started WatcherSet or PathStore
- [ ] Add secrecy crate and use it for all usages of the Github token

### CLI tool

- [ ] Add a command to print the Rust package version (and add it to the Bug issue template on Github)
- [ ] Add an option to create a new gist on the fly, copy settings to it and start using it from now on
- [ ] Handle errors more beautifully, introduce the dedicated Error type if needed
- [ ] Log output through tracing subscriber and/or add coloring of various levels of output messages
      – [ ] Add cross-platform colored plain chars for CLI output instead of colored circle emojis
- [ ] Refactor to get rid of the InteractiveIO trait in favor of BufRead + Write type (trait), see <https://t.me/rustlang_ua/69909/132141>

## CI

- [x] Add matrix to compile for Windows on ARM64
- [x] Speed up the build if possible (caching, Docker images, etc.)
- [ ] ~~Extract "compile" as a separate local Github action~~ No need for that, because "compile" is used only for release.
- [ ] Make non-.rs changes to avoid triggering the "check and test" workflow
- [ ] Optimize binaries size <https://github.com/johnthagen/min-sized-rust>

## Documentation

- [x] Simplify README before public release
- [x] Add eget entry to README part about the installation instructions
- [x] Add docs on how to "sync back" the settings' files from a Gist if they are lost or it's a fresh dev environment
- [x] Verify that there are roughly "How to setup" and "How to run" sections in the README.md file
