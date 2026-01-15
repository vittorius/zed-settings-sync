<!-- markdownlint-disable-file MD033 --><!-- we are OK with inline HTML since we use <kbd> tags -->

# Zed Settings Sync

**Zed Settings Sync** is an extension for [Zed](https://zed.dev) that aims to add support of automatically syncing your user-level config files to a Github Gist using LSP.

ℹ️ This extension doesn't sync project settings files because it's more pragmatic to just check them in the project's VCS repository if needed.

Using LSP is a workaround because of the limited capabilities of current Zed extensions API.

_Such an approach is heavily inspired by [Zed Discord Presence](https://github.com/xhyrom/zed-discord-presence) extension._

## Installation

When a corresponding [Zed extensions repo](https://github.com/zed-industries/extensions) PR is created and merged, you can simply download the extension in <kbd>zed: extensions</kbd>.

Until then, you can use the [dev installation mode](#dev-extension-installation).

## Configuration

### If you already have Zed but you don't have a settings Gist yet

1. Create a Github token with `gist` permission scope ([detailed guide](docs/CREATE_GITHUB_TOKEN.md)).
2. Prepare a Gist ([detailed guide](docs/CREATE_SETTINGS_GIST.md)).
3. Add credentials to your Zed settings file:

```jsonc
{
  "lsp": {
    "settings_sync": {
      "initialization_options": {
        "github_token": "gho_my-shiny-token",
        "gist_id": "deadbeefdeadbeefdeadbeefdeadbeef"
      }
    }
  }
}
```

### If you've installed a fresh Zed and want to pull in your settings from an existing Gist

⚠️ Unfortunately, due to the currently limited functionality of Zed extensions in general, the extension itself cannot load settings from a Github Gist. A CLI tool is provided for that purpose.

Ensure you have your [Github token](docs/CREATE_GITHUB_TOKEN.md) and [Gist ID](docs/CREATE_SETTINGS_GIST.md) at hand.

1. Install [eget](https://github.com/zyedidia/eget)
2. Run `sudo eget vittorius/zed-settings-sync --to=<your local bin directory>`
3. Pick the `zed-settings-sync` binary in the choice provided by eget
4. Run `zed-settings-sync load` and follow the instructions

(Of course, you can download and unpack the binary manually from [Github releases](https://github.com/vittorius/zed-settings-sync/releases))

## Usage

### Syncing to a Github Gist

Given, you've configured everything correctly, now you can:

- edit the settings file ( or <kbd>zed: open settings file</kbd>)
- edit the keymap file ( or <kbd>zed: open keymap file</kbd>)
- edit tasks (<kbd>zed: open tasks</kbd>)
- edit debug tasks (<kbd>zed: open debug tasks</kbd>)

After the file is saved, either manually, or with the auto-save feature, it will be synchronized to the Gist you've specified.

ℹ️ At some point, Zed has added graphical interface for editing Settings and Keymap.
It pops up by default when you run <kbd>zed: open settings</kbd> or <kbd>zed: open keymap</kbd> workbench action.
When using such an editor, click `Edit in settings.json` or `Edit in keymap.json` respectively.
You can go back to the visual editor and use it afterward, **just keep the corresponding JSON settings file open**
for it to be caught by LSP and synchronized appropriately.
Or, of course, you can edit your config files manually, as it was before.

Another approach could be swapping the keymap entries for <kbd>zed: open settings</kbd> or <kbd>zed: open settings file</kbd> (and for keymap file in a similar fashion):

```json
{
  "bindings": {
    "cmd-,": "zed::OpenSettingsFile",
    "alt-cmd-,": "zed::OpenSettings"
  }
}
```

## Troubleshooting

- Open LSP logs (<kbd>dev: open language server logs</kbd>), find `settings_sync` LSP server instance running for the specific settings file, and inspect its log
- File an [issue](https://github.com/vittorius/zed-settings-sync/issues/new) on Github

## Development

### Dev environment setup

Requirements:

- [Git](https://git-scm.com/)
- [Rust](https://rust-lang.org) is required. The easiest way to get [rust](https://rust-lang.org) is by using [rustup](https://rustup.rs).
- [Nextest](https://nexte.st/) test runner ([some tests](common/src/config.rs) rely on it to be run without the need of cross-thread synchronization)
- [iprecommit](https://github.com/iafisher/iprecommit) for Git hooks
  - install `uv`
  - change directory to where you cloned this repository
  - do `uv venv`
  - do `uvx pip install iprecommit`
  - do `uvx precommit install`

### Dev extension installation

1. Clone this repository
2. <kbd>CTRL</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and select <kbd>zed: install dev extension</kbd>
3. Choose the directory where you cloned this repository
4. After installing the extension, reload the workspace (<kbd>workspace: reload</kbd>) to start the LSP server

### Quick feedback loop when working on the LSP server

Run

```shell
cargo xtask-lsp-install
```

to install the LSP server binary from your local repository to the Zed extension directory.
Then, run <kbd>workspace: reload</kbd> action within your Zed instance for your dev extension to catch up the updated LSP server binary.
