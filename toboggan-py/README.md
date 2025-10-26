# toboggan-py

Python bindings for the Toboggan presentation system, enabling real-time multi-client synchronization.

Built with Rust using PyO3 and Maturin for high-performance native Python extensions.

## Features

- Real-time WebSocket-based presentation synchronization
- Cross-client state sharing (navigation, playback control)
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
print(f"Talk: {client.talk}, Slides: {client.slides}, State: {client.state}")
client.next()      # Navigate to next slide
client.previous()  # Navigate to previous slide
```

## API Reference

### `Toboggan(host="localhost", port=8080)`

**Properties:** `talk`, `slides`, `state` (presentation metadata and synchronized state)
**Methods:** `next()`, `previous()` (slide navigation)

Type stubs in `toboggan_py.pyi` provide full IDE support and type checking.

## Development

```bash
cargo fmt && cargo clippy              # Format and lint
maturin develop && python example.py   # Build and test
maturin build --release                # Build release wheel
```

## Troubleshooting

- **Connection fails:** Ensure server is running. Check `http://localhost:8080/health`
- **Build fails:** Verify Rust is installed: `rustc --version` (update with `rustup update`)
- **Import error:** Rebuild with `maturin develop` after code changes

## License

MIT OR Apache-2.0
