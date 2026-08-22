# toboggan-py

Python bindings for the Toboggan presentation system, enabling real-time
multi-client synchronization.

Built with Rust using PyO3 and Maturin for high-performance native Python
extensions.

## Features

- Real-time WebSocket-based presentation synchronization
- Cross-client state sharing (navigation, step reveals, blink)
- Async architecture with Tokio runtime
- Type-safe Python API with full type stubs
- ABI3 wheels for forward compatibility (Python 3.8+)

## Requirements

- Python 3.8 or higher
- Rust toolchain (for building from source)
- Running Toboggan server (see main repository)

## Installation

```bash
# Create virtual environment and install maturin
python -m venv .venv && source .venv/bin/activate
pip install maturin

# Build and install (development mode)
maturin develop
```

Contributing rather than consuming? Use `uv`, which is what the tasks and CI
use and what the committed `uv.lock` pins:

```bash
uv sync --group dev
uv run --no-sync maturin develop --uv
```

`--no-sync` on the second line is not optional: a bare `uv run` re-syncs first
and puts the cached wheel back over the extension `maturin develop` just built,
so you would be testing the previous build.

## Quick Start

```python
from toboggan_py import Toboggan

# As a context manager, so the client's runtime shuts down deliberately.
with Toboggan("localhost", 8080) as client:
    # Access metadata and navigate
    print(f"Talk: {client.talk}, Slides: {len(client.slides)}")
    client.next()  # Navigate to next slide
    client.previous()  # Navigate to previous slide
    client.goto(12)  # Jump to the slide numbered 12, counting from 1

    print(client.state)  # correct straight away — no sleep needed
    print(client.state.is_last_slide)  # the state knows its own deck
```

The bindings report over Python's `logging` rather than printing, so nothing
lands on your stdout unless you ask:

```python
import logging

logging.basicConfig(level=logging.DEBUG)  # socket, deck reloads, clients
```

Navigation is synchronous: a call returns once the server has applied it, so
the `state` you read next is the state that call produced. A command the server
refuses raises rather than quietly doing nothing.

## Presenter and audience

A client never claims a role — it offers a presenter token and the server
decides. A connection from the machine running the server presents; a
connection from anywhere else presents only if it carries the token:

```python
client = Toboggan("192.168.1.20", 8080, presenter_token="s3cret")
if not client.is_presenter:
    print(f"watching only ({client.role}) — navigation would raise")
```

Navigating without the right to do so raises `PermissionError`, so a script
that assumed it was presenting stops where it went wrong rather than reporting
success over a deck that never moved.

`presenter_token` falls back to the `TOBOGGAN_PRESENTER_TOKEN` environment
variable, which is where `toboggan tui` and `toboggan desktop` read theirs. See
[SECURITY.md](../SECURITY.md) in the main repository.

## API Reference

### `Toboggan(host="localhost", port=8080, presenter_token=None)`

| | |
| --- | --- |
| Properties | `talk`, `slides`, `state`, `role`, `is_presenter` |
| Navigation | `next()`, `previous()`, `first()`, `last()`, `goto(n)` |
| Steps | `next_step()`, `previous_step()` |
| Other | `blink()`, `clients()` |

`talk` carries the deck's `title`, `date`, `lang`, `footer`, `head`, `titles`,
`step_counts` and per-slide `durations`. `slides` supports `len()`, indexing and
iteration, and each `Slide` has `kind`, `title`, `body`, `notes`, `duration` and
`hidden_in`. `state` reports `is_init` / `is_running` / `is_done`, the current
`slide` and `step`, and `is_first_slide(total)` / `is_last_slide(total)`.

`clients()` is presenter-only on the server — it reports names, roles and IP
addresses — and raises `PermissionError` otherwise. Every navigation method
raises the same on an audience connection, `RuntimeError` when the server
rejects the command (an out-of-range `goto`, a deck with no slides), and
`ConnectionError` when the server cannot be reached.

Type stubs in `toboggan_py.pyi` provide full IDE support and type checking.

## Development

From the repository root:

```bash
mise check:py  # cargo fmt + clippy, ruff, mypy, stubtest, and the test suite
mise test:py   # just the tests
```

From this directory:

```bash
maturin build --release  # build a release wheel
```

`uv` is required for both tasks — they fail rather than skip without it, so a
green summary always means the checks actually ran.

`mise check` at the repository root runs `check:py` along with the Rust, web and
iOS lanes. The crate is excluded from the cargo workspace, so the root `cargo`
commands do not reach it — that is what these tasks are for.

The tests start a real server and drive it, because nothing about a client can
be tested honestly without one. They pick a free port, so they will not disturb
a deck you already have running — or set `TOBOGGAN_PY_TEST_PORT` to pin one.
Set `TOBOGGAN_BIN` to a prebuilt `toboggan` to skip the `cargo run` they
otherwise fall back to; `mise test:py` does this for you when the workspace has
already built one.

Some tests need a non-loopback address, because the server grants the presenter
role to loopback unconditionally and an audience client cannot be made over
`localhost` at all. They skip without one. Set `TOBOGGAN_PY_STRICT=1` — as CI
does — to make that a failure instead: a runner that silently drops the whole
role suite should not report success.

## Troubleshooting

- **Connection fails:** Ensure server is running. Check
  `http://localhost:8080/health`
- **Navigation raises `PermissionError`:** check `client.is_presenter`. Across
  the network, an audience client's commands are refused by design — pass a
  presenter token.
- **Build fails:** Verify Rust is installed: `rustc --version` (update with
  `rustup update`)
- **Import error:** Rebuild with `maturin develop` after code changes

## License

MIT OR Apache-2.0
