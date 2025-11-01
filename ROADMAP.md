# Roadmap

## Development

- [x] Test the release is working
- [x] Test the extension installation and downloading the released LSP binary from Github is working
- [x] Write a test a script to locally build the LSP binary and copy it to the Zed's folder for the extension (see Zed Discord Presence LSP server info for the full path, also check out the Zed sources, namely paths.rs)
- [ ] After implementing naive changes persistence (unbounded queue), seek the ways to improve it

## CI

- [ ] Enable ["cancel-in-progress"](https://share.google/Vk4zJKCbkerc5BAfC) for CI builds
- [ ] Add matrix to compile for Windows on ARM64
- [ ] Speed up the build if possible (caching, Docker images, etc.)
- [ ] Extract "compile" as a separate local Github action

## Documentation

- [ ] Add docs on how to "sync back" the settings' files from a Gist if they are lost or it's a fresh dev environment
