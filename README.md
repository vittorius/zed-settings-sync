<!-- markdownlint-disable-file MD033 --><!-- we are OK with inline HTML since we use <kbd> tags -->

# Zed Settings Sync

**Zed Settings Sync** is an extension for [Zed](https://zed.dev) that aims to add support of automatically syncing your user-level config files to a Github Gist using LSP.

ℹ️ This extension doesn't sync local settings files (from the project dir) because it's more pragmatic to just check them in the project's VCS repository if needed.

Using LSP is a workaround because of the limited capabilities of current Zed extensions API.

_Such an approach is heavily inspired by [Zed Discord Presence](https://github.com/xhyrom/zed-discord-presence) extension._

## Requirements

[rust](https://rust-lang.org) is required for installing this extension. \
The easiest way to get [rust](https://rust-lang.org) is by using [rustup](https://rustup.rs).

## How to install?

### Dev installation

1. Clone this repository
2. <kbd>CTRL</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and select <kbd>zed: install dev extension</kbd>
3. Choose the directory where you cloned this repository
4. After installing the extension, reload the workspace (<kbd>workspace: reload</kbd>) to start the LSP server
5. Enjoy :)

### Normal installation

When a corresponding [Zed extensions repo](https://github.com/zed-industries/extensions) PR is created and merged, you can simply download the extension in <kbd>zed: extensions</kbd>.

## How to configure?

### Prepare a Github token

#### Using Github CLI

This is the easiest way.

1. Install the official [Github CLI](https://github.com/cli/cli#installation)
2. [Login](https://cli.github.com/manual/gh_auth_login) to Github using it
3. Ensure your token has the `gist` OAuth scope (it should, by default):

```shell
gh auth status
```

<!-- markdownlint-disable MD029 -->

4. Copy your token to the clipboard and paste it into your configuration file:

macOS:

```shell
gh auth token | pbcopy
```

Linux:

```shell
gh auth token | xclip -selection clipboard
```

Windows:

```shell
gh auth token | clip
```

5. Paste it into your Settings Sync configuration:

```jsonc
{
  // ...
  "lsp": {
    // ...
    "settings_sync": {
      "initialization_options": {
        "github_token": "<your Github token>"
        // ...
      }
    }
  }
}
```

#### Create a token yourself

1. Create a new token [at Github](https://github.com/settings/personal-access-tokens/new).
2. Ensure it has **Gists** permission under the **Account**.
3. Perform all of the steps from the previous section to land this token into your Settings Sync LSP server configuration.

### Prepare a Gist

You need to create a Gist or have an existing one. If you're creating a new one, remember that it cannot be empty or contain zero-sized files.
So, to create a Gist for our purposes, again, we have 2 options.

#### Github CLI

macOS / Linux:

```shell
echo "// Zed Settings\n\n{\n}\n" | gh gist create -f settings.json -d "Zed Settings"
```

Windows:

```shell
echo //^ Zed^ Settings| gh gist create -f settings.json -d "Zed Settings"
```

#### curl

macOS / Linux:

```shell
curl -X POST -H "Authorization: token <your Github token>" -H "Content-Type: application/json" -d '{"description": "Zed Settings", "public": false, "files": {"settings.json": {"content": "// Zed Settings\n\n{\n}\n"}}}' https://api.github.com/gists
```

Windows:

```shell
curl.exe -X POST -H "Authorization: token <your Github token>" -H "Content-Type: application/json" -d "{\"description\":\"Zed Settings\",\"public\":false,\"files\":{\"settings.json\":{\"content\":\"// Zed Settings\n\n{\n}\n\"}}}" https://api.github.com/gists
```

#### Insert Gist ID into your Settings Sync configuration

5. Paste it into your Settings Sync configuration:

```jsonc
{
  // ...
  "lsp": {
    // ...
    "settings_sync": {
      "initialization_options": {
        "gist_id": "<your Gist Id>"
        // ...
      }
    }
  }
}
```

### Example configuration

```jsonc
{
  "lsp": {
    "settings_sync": {
      "initialization_options": {
        "github_token": "gho_nA8tK4GxW9eR1bY0uZqT7sL2pCjD5vFhE",
        "gist_id": "e565898c6f664eb916c54de1e99ebe74"
      }
    }
  }
}
```

## How to use?

### Syncing to a Github Gist

Given, you've configured everything correctly, now you can:

- edit the settings file ( or <kbd>zed: open settings file</kbd>)
- edit the keymap file ( or <kbd>zed: open keymap file</kbd>)
- edit tasks (<kbd>zed: open tasks</kbd>)
- edit debug tasks (<kbd>zed: open debug tasks</kbd>)

After the file is saved, either manually, or triggered by the auto-save feature, it will be synchronized to the Gist you've specified.

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

### Loading settings back from a Github Gist

⚠️ Unfortunately, the extension in its current implementation does not provide an action or a command to load settings from a Github Gist.
This is because of the limited functionality of Zed extensions in general. But, a CLI tool (TBD) is provided to download settings from the Gist.

- TODO: add a section about the binary CLI crate to download settings from the Gist

## Troubleshooting

- Open LSP logs (<kbd>dev: open language server logs</kbd>), find Settings Sync LSP server, and inspect its log
- File an [issue](https://github.com/vittorius/zed-settings-sync/issues/new) on Github

## Development

- TODO: install rust and other components via rustup
- TODO: install iprecommit (install uv, do uv venv, do ux pip install iprecommit)
- TODO: other neccessary setup
