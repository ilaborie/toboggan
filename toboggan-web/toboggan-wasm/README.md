# toboggan-wasm

The web client, in Rust, compiled to WebAssembly. It is what `/run` and
`/presenter` load, and it is embedded into the `toboggan` binary at compile time.

## The JS surface

Two exported functions. That is the whole API:

```ts
import init, { start_app, start_presenter_app, AppConfig } from "./pkg/toboggan_wasm";

await init();

const config = new AppConfig();          // defaults to this page's own origin
start_app(config, document.querySelector("main"));
```

| Export | Renders |
| --- | --- |
| `start_app(config, element)` | The deck — what goes on the projector |
| `start_presenter_app(config, element)` | The presenter view: notes, next slide, timer |
| `AppConfig` | `api_base_url`, `websocket`, and an optional keymap |
| `WebSocketConfig` | URL, `max_retries`, `initial_retry_delay`, `max_retry_delay` |

`AppConfig::new()` reads the page's own origin, so a client served by the server
it talks to needs no configuration at all. The TypeScript side
(`../src/boot.ts`) is a thin wrapper that lets environment variables override
that for local development.

Both entry points build the **same** application; `start_presenter_app` differs
only in what it draws around the current slide. A separate client would have had
to reimplement reconnection, role handling and slide fetching, and would have
drifted from the first within a release.

## Components

Each is a custom element with its own shadow root, so a deck's CSS and the
client's chrome cannot reach into one another:

| Component | What it is |
| --- | --- |
| `slide` | Renders one slide, including its reveal steps |
| `presenter` | The two-pane presenter layout and its status strip |
| `terminal` | An embedded terminal, from a `<!-- term: … -->` directive |
| `quake_terminal` | The drop-down terminal overlay, toggled with `` ` `` |
| `help` | The shortcut dialog, toggled with `F1` |
| `footer` | The deck's footer and progress bar |
| `toast` | Connection and error messages |

Custom properties inherit across a shadow boundary and selectors do not, which
is why a deck themes slides through CSS variables rather than descendant rules.

## Keys

`←` `→` slide, `↑` `↓` `Space` step, `PageUp`/`PageDown`/`Backspace` for a
presenter remote, `Home`/`End`, digits then `Enter` to jump, `b` blink, `f`
fullscreen, `.` and `w` to blank the screen, `` ` `` for the quake terminal, and
`F1` for help. The help dialog is generated from the keymap, so a new binding
documents itself.

## Building

```bash
mise build:web       # wasm-pack + vite → toboggan-web/dist, which the server embeds
```

> [!NOTE]
> This crate is in a **separate Cargo workspace** (`toboggan-web`). `--workspace`
> from the repository root does not reach it, so lint it where it lives:
>
> ```bash
> cargo clippy --all-targets --target wasm32-unknown-unknown -- -D warnings
> ```
>
> `wasm32-unknown-unknown` is the only target it is ever built for; a host-target
> lint would be checking code the browser never runs.

## License

MIT or Apache-2.0, at your option.
