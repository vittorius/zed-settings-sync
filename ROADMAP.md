# Roadmap

## Development

### Overall

- [x] Test the release is working
- [x] Test the extension installation and downloading the released LSP binary from Github is working
- [x] Write a test a script to locally build the LSP binary and copy it to the Zed's folder for the extension (see Zed Discord Presence LSP server info for the full path, also check out the Zed sources, namely paths.rs)
- [x] Forbid the usage of unwrap and expect for Option, Result
- [x] Use serde_json from zed_extension_api, not directly
- [~] **Add unit and integration tests**
- [ ] Test all error-returning code paths and ensure that all error conditions are either properly logged and/or reported back to Zed in form or a JSON-RPC error response
- [ ] Revamp README before publishing the extension to make it easier to consume and start using the extension immediately
- [ ] Manually save a settings file on its open (before adding to the watch list) to handle the case when the LSP server is restarted after the initialization_options are changes in settings.json file.
- [ ] Ensure that restarting or shutting down the LSP server doesn't prevent the last coming updates from getting synced; otherwise, mitigate that
- [ ] After implementing naive changes persistence (sync files as FS events come), seek the ways to improve it (e.g. queuing events)
      – [ ] (experimental) Rewrite the LSP server to use structured async concurrency, thread-per-code async runtime, and get rid of Arc's around every data structure
- [ ] Add secrecy crate and use it for all usages of the Github token
- [ ] To support multiple sync providers, implement a SyncClient trait and use it for all sync operations. Move it to the "common" crate.
- [ ] Report a sync error in a visible way (crash the server? know how to report an LSP error inside Zed?)
- [ ] Backup installed themes to the gist automatically

### CLI tool

- [ ] Handle errors more beautifully, introduce the dedicated Error type if needed
- [ ] Log output through tracing subscriber and/or add coloring of various levels of output messages
      – [ ] Add cross-platform colored plain chars for CLI output instead of colored circle emojis
- [ ] Refactor to get rid of the InteractiveIO trait in favor of BufRead + Write type (trait), see <https://t.me/rustlang_ua/69909/132141>
- [ ] Add an option to create a new gist on the fly, copy settings to it and start using it from now on

## CI

- [ ] Enable ["cancel-in-progress"](https://share.google/Vk4zJKCbkerc5BAfC) for CI builds
- [ ] Add matrix to compile for Windows on ARM64
- [ ] Speed up the build if possible (caching, Docker images, etc.)
- [ ] Extract "compile" as a separate local Github action
- [ ] Optimize binaries size <https://github.com/johnthagen/min-sized-rust>

## Documentation

- [ ] Simplify README before public release
- [ ] Add eget entry to README part about the installation instructions
- [ ] Add docs on how to "sync back" the settings' files from a Gist if they are lost or it's a fresh dev environment
- [ ] Verify that there are roughly "How to setup" and "How to run" sections in the README.md file
