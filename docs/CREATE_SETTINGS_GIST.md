# Prepare a Gist

You need to create a Gist or have an existing one. If you're creating a new one, remember that it cannot be empty or contain zero-sized files.
So, to create a Gist for our purposes, again, we have 2 options.

## Github CLI

macOS / Linux:

```shell
echo "// Zed Settings\n\n{\n}\n" | gh gist create -f settings.json -d "Zed Settings"
```

Windows:

```shell
echo //^ Zed^ Settings| gh gist create -f settings.json -d "Zed Settings"
```

## curl

macOS / Linux:

```shell
curl -X POST -H "Authorization: token <your Github token>" -H "Content-Type: application/json" -d '{"description": "Zed Settings", "public": false, "files": {"settings.json": {"content": "// Zed Settings\n\n{\n}\n"}}}' https://api.github.com/gists
```

Windows:

```shell
curl.exe -X POST -H "Authorization: token <your Github token>" -H "Content-Type: application/json" -d "{\"description\":\"Zed Settings\",\"public\":false,\"files\":{\"settings.json\":{\"content\":\"// Zed Settings\n\n{\n}\n\"}}}" https://api.github.com/gists
```
