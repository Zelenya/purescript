# Zed PureScript (experimental version)

A [PureScript](https://www.purescript.org/) extension for [Zed](https://zed.dev).

## (EXPERIMENTAL) Language servers

This extension ships two language servers:

- **`purescript-language-server`** — the mature, node, `purs ide`-based
  [nwolverson/purescript-language-server](https://github.com/nwolverson/purescript-language-server).
  Installed automatically via npm.
- **`purescript-alexandrite`** — the new (pre-1.0) PureScript analyzer
  [purefunctor/purescript-alexandrite](https://github.com/purefunctor/purescript-alexandrite),
  built on an incremental, query-based engine for fast interactive editing.
  Prebuilt binaries are downloaded from GitHub releases. Doesn't provide full set of features yet.

> [!IMPORTANT]
> **After installing, both servers are going to run at the same time!** 
> 
> This extension does not — and cannot — pick one for you. 
> You have to configure `language_servers` to choose which server(s) to run.
>
> If you run two servers, you'll see duplicated results (e.g., **two popups on hover**),
> but `purescript-alexandrite` acts as the primary server for go-to-definition, references, and rename. 
> If you only want one, configure it as shown below.

### Choosing which server(s) run

When `language_servers` is unset it defaults to `["..."]`, which enables every server registered for the language, 
in alphabetical order by id (`purescript-Alexandrite` comes before `purescript-Language-server`). However, overlapping capabilities stack (zed has no per-feature routing).

You can select servers per language with [`language_servers`](https://zed.dev/docs/configuring-languages#choosing-language-servers).

`settings.json` examples:

```jsonc
// only alexandrite (recommended while evaluating it)
{ "languages": { "PureScript": { "language_servers": ["purescript-alexandrite"] } } }
```

```jsonc
// only the node server (the full-featured default experience)
{ "languages": { "PureScript": { "language_servers": ["purescript-language-server"] } } }
```

```jsonc
// both, alexandrite first (you will get stacked hovers)
{ "languages": { "PureScript": { "language_servers": ["purescript-alexandrite", "purescript-language-server"] } } }
```

```jsonc
// explicitly disable alexandrite, keep only the node server (easier to switch back and forth)
{ "languages": { "PureScript": { "language_servers": ["!purescript-alexandrite", "purescript-language-server"] } } }
```

`["purescript-alexandrite"]` is exhaustive (listing only one server disables the other); `!name` is for the case shown above.

**Tip**: put this in a project-local `.zed/settings.json` to pick a server per project, 
and note that editing the setting hot-reloads the servers — no restart needed.

### Alexandrite: project sources / binary configuration

Alexandrite discovers source files from a `spago.lock` produced by [`spago@next`](https://github.com/purescript/spago). 
For classic-spago or non-spago projects, give it a command that prints source paths/globs on stdout via `--source-command`:

```jsonc
{
    "lsp": {
        "purescript-alexandrite": {
            "binary": {
                // optional: use a self-installed binary instead of the download
                "path": "/usr/local/bin/purescript-alexandrite",
                "arguments": ["--source-command", "spago sources"]
            }
        }
    }
}
```

Note that the required `--stdio` flag is added automatically (and skipped ifyou include it in `arguments` yourself), 
so `arguments` only needs your extraflags.

The `purescript-alexandrite` binary is resolved in order: an explicit `binary.path`, then a `purescript-alexandrite` found on your `$PATH`, then a download of the pinned release.

> **Linux ARM (aarch64):** upstream publishes no prebuilt binary yet. Install alexandrite yourself (`cargo install`) and set `binary.path` as above.

## Configuration

> Zed has no per-extension settings UI. Everything below goes into your `settings.json` (zed passes `lsp.<server>.settings` and `initialization_options` through to the language server verbatim without validation or autocompletion). 
> The authoritative list of server options lives in each language server's own documentation:
> ([purescript-language-server](https://github.com/nwolverson/purescript-language-server#configuration),
> [purescript-alexandrite](https://github.com/purefunctor/purescript-alexandrite)).

### Formatting

Formatting is provided only by **`purescript-language-server`** (alexandrite has no formatter). It is **off until you choose a tool** via `purescript.formatter`. Options: `purs-tidy` (recommended), `pose`, `purty`.

For example, install the tool so the server can find it on `$PATH`:

```bash
npm install -g purs-tidy
```

Then enable it:

```jsonc
{
    "languages": {
        "PureScript": {
            "formatter": "language_server",
            "format_on_save": "on" // optional
        }
    },
    "lsp": {
        "purescript-language-server": {
            "settings": {
                "purescript": {
                    "formatter": "purs-tidy",
                    "addNpmPath": true // only if purs-tidy is a local npm dep, not on $PATH
                }
            }
        }
    }
}
```

### Environment Variables

You can specify arguments as well as environment variables to pass to the language server by configuring the `lsp` settings in your Zed settings. This can be helpful for Nix setups or other environments where you need to customize the `PATH` or other variables so that the language server can find the purescript binary to invoke for the LSP support.

Example configuration in your `settings.json`:

```json
{
    "lsp": {
        "purescript-language-server": {
            "binary": {
                "env": {
                    "PATH": "/nix/store/gw58kr741a9ddmv3xn47llc7i07jbbvr-purescript-0.15.15/bin"
                }
            }
        }
    }
}
```

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.
