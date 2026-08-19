# toboggan-server

The [Toboggan](https://github.com/ilaborie/toboggan) server: an [axum] service
that serves a deck over REST, keeps every connected client on the same slide over
a WebSocket, and hosts the embedded web client.

> [!IMPORTANT]
> Reach this through the unified
> [`toboggan`](../toboggan) command — `toboggan -p my-talk` to build a folder in
> memory and serve it with live reload, or `toboggan serve -p talk.toml` to serve
> an already-built file.

[axum]: https://github.com/tokio-rs/axum

## Routes

| Method | Path | What it is |
| --- | --- | --- |
| `GET` | `/` | Homepage, linking to everything below |
| `GET` | `/run` | The deck — the embedded web client |
| `GET` | `/presenter` | Presenter view: notes, next slide, timer |
| `GET` | `/slides` | Thumbnail overview (needs `--thumbnails-dir`) |
| `GET` | `/guide` | The bundled authoring guide |
| `GET` | `/guide/public/{*path}` | The guide deck's own assets |
| `GET` | `/download.pdf` | The deck rendered to PDF (needs `typst`) |
| `GET` | `/doc` | OpenAPI reference, rendered with Scalar |
| `GET` | `/health` | Liveness check |
| `GET` | `/api/talk` | The whole deck |
| `GET` | `/api/slides` | Every slide, with step counts |
| `GET` | `/api/slides/{index}` | One slide, by 0-based index |
| `POST` | `/api/command` | Send a `Command` over HTTP 🔒 |
| `GET` | `/api/clients` | Connected clients 🔒 |
| `GET` | `/api/ws` | WebSocket: commands in, notifications out |
| `GET` | `/api/terminal` | WebSocket for an embedded terminal 🔒 |
| `GET` | `/public/{*path}` | The deck's assets (needs `--public-dir`) |
| `GET` | `/overview/{*path}` | Generated thumbnail assets |

🔒 **presenter only.** A connection from the machine running the server presents;
a connection from elsewhere presents only if it carries the presenter token. See
[SECURITY.md](../SECURITY.md) — `/api/terminal` spawns a real shell, so this is
the difference between a demo and a stranger on the conference wifi getting one.

The gate is an axum extractor (`Presenter`) rather than a check inside each
handler, so a privileged route that forgets to ask for it reads as unprivileged
in its own signature.

## State

The presentation state machine has exactly three states:

```
Init  ──►  Running { current, current_step }  ──►  Done { current, current_step }
```

`Init` is before anything has been shown; `Running` carries the current slide and
which reveal within it is showing; `Done` is past the last slide. There is no
pause: a deck that is not moving is simply a `Running` state nobody is sending
commands about.

Every state change is broadcast to every client as
`Notification::State { state }`, which is what keeps a browser, a terminal and a
phone showing the same thing. The full protocol lives in
[`toboggan-core`](../toboggan-core).

## Configuration

Every setting is a flag on `toboggan serve`, an entry under `[serve]` in
`toboggan.toml`, and a `TOBOGGAN_*` environment variable.

| Flag | Env var | Default |
| --- | --- | --- |
| `--host` | `TOBOGGAN_HOST` | `127.0.0.1` |
| `--port` | `TOBOGGAN_PORT` | `8080` |
| `--max-clients` | `TOBOGGAN_MAX_CLIENTS` | `100` |
| `--public-dir` | `TOBOGGAN_PUBLIC_DIR` | — |
| `--thumbnails-dir` | `TOBOGGAN_THUMBNAILS_DIR` | — |
| `--shell` | `TOBOGGAN_SHELL` | `$SHELL`, else `sh` |
| `--allowed-origins` | `TOBOGGAN_CORS_ORIGINS` | any origin |
| `--presenter-token` | `TOBOGGAN_PRESENTER_TOKEN` | — |
| `--open` | `TOBOGGAN_OPEN` | `false` |
| `--open-presenter` | `TOBOGGAN_OPEN_PRESENTER` | `false` |

The bind address defaults to loopback. Open it up and the server says what that
means on startup — that remote clients are read-only, or that a token is in play.

## Using it as a library

```rust,ignore
use toboggan_core::Talk;
use toboggan_server::{ServerSettings, launch_with_talk};

// Serve a talk you already have in memory. `None` means no file watching;
// pass a `WatchConfig` to hot-swap the deck when its source changes.
launch_with_talk(talk, settings, None).await?;
```

`launch_with_talk` is the shared serving core: `launch` uses it after reading a
`.toml` file, and the unified CLI's build-and-serve uses it with a talk parsed
from a folder plus a recursive watcher.

Also public: `routes` / `routes_with_cors` to mount the router yourself,
`TobogganState`, `PresenterAuth`, `WatchConfig` / `start_watch_task`, and
`openapi_json()` for the bundled OpenAPI document (`toboggan openapi` prints it).

> [!NOTE]
> The crate embeds `toboggan-web/dist` at compile time with `rust-embed`, and its
> `build.rs` **fails when that folder is missing**. Run `mise build:web` before
> building this crate for the first time, and again after changing the web
> client.

## License

MIT or Apache-2.0, at your option.
