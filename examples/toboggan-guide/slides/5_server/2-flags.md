+++
title = "Server flags"
classes = ["no_title", "wide"]
+++

# Server flags

| Flag | Env var | What it does |
|---|---|---|
| `--host` | `TOBOGGAN_HOST` | Bind address (default `127.0.0.1`) |
| `--port` | `TOBOGGAN_PORT` | Port (default `8080`) |
| `--watch` | `TOBOGGAN_WATCH` | Live-reload when the `.toml` changes |
| `--public-dir` | `TOBOGGAN_PUBLIC_DIR` | Serve assets at `/public/` |
| `--shell` | `TOBOGGAN_SHELL` | Shell for embedded terminals |
| `--allowed-origins` | `TOBOGGAN_CORS_ORIGINS` | Comma-separated CORS origins |
| `--max-clients` | `TOBOGGAN_MAX_CLIENTS` | Concurrent client cap |

<!-- pause -->

```console
$ toboggan-server \
    --public-dir ./public/ \
    --shell /opt/homebrew/bin/fish \
    --watch \
    ./my-talk.toml
```

> [!TIP]
> `--shell` picks which shell the live terminals spawn — point it at `fish`
> to show off your real prompt, or `sh` for a clean, portable demo.
