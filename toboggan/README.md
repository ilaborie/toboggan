# toboggan

**The `toboggan` command** — the entry point for everything, and the entry
point for everything [Toboggan](https://github.com/ilaborie/toboggan) does. Every
other crate here is a library this one dispatches to.

```bash
toboggan -p my-talk     # build the folder in memory and serve it, reloading on save
```

`-p/--path` defaults to `.`, so `cd my-talk && toboggan` works too.

## Commands

| Command | Purpose |
| --- | --- |
| `toboggan -p <folder>` | Build + serve with a folder watch (the default action) |
| `toboggan watch -p <folder>` | The default action, named explicitly |
| `toboggan build -p <folder> -o out.{toml,json,yaml,html,typ}` | Build to a file |
| `toboggan serve -p <talk.toml>` | Serve an already-built talk (a **file**) |
| `toboggan new -p <dir>` | Scaffold a deck (jj repo + `.mcp.json` + skill) |
| `toboggan lint -p <folder>` | Lint the deck |
| `toboggan stats -p <folder>` | Word counts and duration estimates |
| `toboggan pdf -p <folder>` | Render a PDF (needs `typst`) |
| `toboggan thumbnails -p <folder>` | Per-slide PNGs + `overview.html` + search |
| `toboggan tui` | Present from a terminal, against a running server |
| `toboggan desktop` | Present from a native window |
| `toboggan openapi` | Print the bundled OpenAPI document |
| `toboggan mcp [serve\|init]` | MCP authoring server, or install it |
| `toboggan skills` | Install the authoring skill for Claude Code |
| `toboggan completion <shell>` | Print a shell completion script |

## Configuration

Anything you can pass as a flag can live in a `toboggan.toml` instead. Files are
read from the deck directory, then each parent directory, then
`~/.config/toboggan/config.toml`, so a repository of several decks can share a
house style and one deck can override it.

```
CLI flag  >  TOBOGGAN_* env var  >  nearest toboggan.toml  >  … >  user global  >  default
```

An unknown key is an error rather than a silent no-op. `toboggan new` writes a
`toboggan.toml` listing every setting, commented out with its default — that file
is the reference for what can be configured.

`default-command` decides what a bare `toboggan` does; it is `serve`, which is
what makes the one-word invocation above the whole authoring loop.

## What it dispatches to

| Crate | Does |
| --- | --- |
| [`toboggan-cli`](../toboggan-cli) | Parse a folder; render TOML/JSON/YAML/HTML/Typst; thumbnails |
| [`toboggan-server`](../toboggan-server) | Serve, watch, WebSocket, presenter view, PDF |
| [`toboggan-lint`](../toboggan-lint) | `lint` |
| [`toboggan-stats`](../toboggan-stats) | `stats` |
| [`toboggan-mcp`](../toboggan-mcp) | `mcp` |
| [`toboggan-tui`](../toboggan-tui) / [`toboggan-desktop`](../toboggan-desktop) | `tui`, `desktop` |

Errors are reported with [`miette`], so a bad slide points at the file and the
line rather than at a stack trace.

## Installing

Releases publish `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` binaries.

> [!WARNING]
> **Do not run `cargo install toboggan`.** That name on crates.io belongs to an
> unrelated project. This one is not published there; install from a release, or
> `cargo install --path toboggan` after `mise build:web`.

See the [root README](../README.md) for details.

## License

MIT or Apache-2.0, at your option.

[`miette`]: https://github.com/zkat/miette
