# Create a Github token

## Using Github CLI

This is the easiest way.

1. Install the official [Github CLI](https://github.com/cli/cli#installation)
2. [Login](https://cli.github.com/manual/gh_auth_login) to Github using it
3. Ensure your token has the `gist` OAuth scope (it should, by default):

```shell
gh auth status
```

<!-- markdownlint-disable MD029 -->

4. Copy your token to the clipboard:

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

## Manually

1. Create a new token [at Github](https://github.com/settings/personal-access-tokens/new).
2. Ensure it has **Gists** permission under the **Account**.
3. Perform all of the steps from the previous section to land this token into your Settings Sync LSP server configuration.
