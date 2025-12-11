# Roadmap

## Development

### Overall

- [x] Test the release is working
- [x] Test the extension installation and downloading the released LSP binary from Github is working
- [x] Write a test a script to locally build the LSP binary and copy it to the Zed's folder for the extension (see Zed Discord Presence LSP server info for the full path, also check out the Zed sources, namely paths.rs)
- [x] Forbid the usage of unwrap and expect for Option, Result
- [~] **Add unit and integration tests**
- [ ] Test all error-returning code paths and ensure that all error conditions are either properly logged and/or reported back to Zed in form or a JSON-RPC error response
- [ ] Manually save a settings file on its open (before adding to the watch list) to handle the case when the LSP server is restarted after the initialization_options are changes in settings.json file.
- [ ] Ensure that restarting or shutting down the LSP server doesn't prevent the last coming updates from getting synced; otherwise, mitigate that
- [ ] After implementing naive changes persistence (sync files as FS events come), seek the ways to improve it (e.g. queuing events)
      – [ ] (experimental) Rewrite the LSP server to use structured async concurrency, thread-per-code async runtime, and get rid of Arc's around every data structure
- [ ] Use serde_json from zed_extension_api, not directly
- [ ] Add secrecy crate and use it for all usages of the Github token

### CLI tool

- [ ] Handle errors more beautifully, introduce the dedicated Error type if needed
- [ ] Log output through tracing subscriber and/or add coloring of various levels of output messages

## CI

- [ ] Enable ["cancel-in-progress"](https://share.google/Vk4zJKCbkerc5BAfC) for CI builds
- [ ] Add matrix to compile for Windows on ARM64
- [ ] Speed up the build if possible (caching, Docker images, etc.)
- [ ] Extract "compile" as a separate local Github action

## Documentation

- [ ] Add docs on how to "sync back" the settings' files from a Gist if they are lost or it's a fresh dev environment
- [ ] Verify that there are roughly "How to setup" and "How to run" sections in the README.md file
