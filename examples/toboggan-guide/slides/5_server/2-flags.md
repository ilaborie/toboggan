+++
title = "Server flags"
classes = ["no_title", "wide"]
+++

# Server flags

| Flag | Env var | What it does |
|---|---|---|
| `--host` | `TOBOGGAN_HOST` | Bind address (default `127.0.0.1`) |
| `--port` | `TOBOGGAN_PORT` | Port (default `8080`) |
| `--public-dir` | `TOBOGGAN_PUBLIC_DIR` | Serve assets at `/public/` |
| `--thumbnails-dir` | `TOBOGGAN_THUMBNAILS_DIR` | Serve an overview at `/overview/` |
| `--shell` | `TOBOGGAN_SHELL` | Shell for embedded terminals |
| `--allowed-origins` | `TOBOGGAN_CORS_ORIGINS` | Comma-separated CORS origins |
| `--max-clients` | `TOBOGGAN_MAX_CLIENTS` | Concurrent client cap |
| `--presenter-token` | `TOBOGGAN_PRESENTER_TOKEN` | Let a remote client drive |
| `--open` | `TOBOGGAN_OPEN` | Open a browser once ready |
| `--watch` | — | Reload when the `.toml` changes |

<!-- pause -->

```console
$ toboggan serve -p ./my-talk.toml \
    --public-dir ./public/ \
    --shell /opt/homebrew/bin/fish \
    --watch
```

> [!TIP]
> `--shell` picks which shell the live terminals spawn — point it at `fish`
> to show off your real prompt, or `sh` for a clean, portable demo.

> [!WARNING]
> Opening `--host` to the network makes the deck read-only for everyone but
> this machine. Add `--presenter-token` to let your phone drive it too.

<!-- notes -->
Any of these can live in `toboggan.toml` under `[serve]` instead of being typed
every time. `--watch` is the one flag with no env var — watching a built file is
a dev-loop choice, not deployment configuration.

The token exists because `--shell` is real: the embedded terminals spawn a shell
on *this* machine, and the room should not be able to ask for one.
