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

## Quick Start

```python
from toboggan_py import Toboggan

client = Toboggan("localhost", 8080)

# Access metadata and navigate
print(f"Talk: {client.talk}, Slides: {len(client.slides)}, State: {client.state}")
client.next()      # Navigate to next slide
client.previous()  # Navigate to previous slide
client.goto(12)    # Jump to the slide numbered 12
```

## Presenter and audience

A client never claims a role — it offers a presenter token and the server
decides. A connection from the machine running the server presents; a
connection from anywhere else presents only if it carries the token:

```python
client = Toboggan("192.168.1.20", 8080, presenter_token="s3cret")
if not client.is_presenter:
    print(f"watching only ({client.role}) — the server refuses navigation")
```

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
addresses — and raises `ConnectionError` otherwise.

Type stubs in `toboggan_py.pyi` provide full IDE support and type checking.

## Development

```bash
cargo fmt && cargo clippy              # Format and lint
maturin develop && python example.py   # Build and test
maturin build --release                # Build release wheel
```

## Troubleshooting

- **Connection fails:** Ensure server is running. Check
  `http://localhost:8080/health`
- **Navigation does nothing:** check `client.is_presenter`. Across the network,
  an audience client's commands are refused by design.
- **Build fails:** Verify Rust is installed: `rustc --version` (update with
  `rustup update`)
- **Import error:** Rebuild with `maturin develop` after code changes

## License

MIT OR Apache-2.0
