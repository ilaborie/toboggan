# Toboggan 🛷

A modern, multi-platform presentation system built in Rust with real-time synchronization across devices.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

## Overview

Toboggan is a presentation system that allows you to create, serve, and control slide-based presentations across multiple platforms. Write your slides in Markdown or TOML, serve them via a WebSocket-enabled server, and present from any client - web browser, terminal, desktop app, or mobile device.

**Note**: This is an educational and fun project created to explore Rust's capabilities across different platforms - from embedded systems to web browsers. While fully functional, it's designed primarily for learning and experimentation rather than production use. It's a playground to demonstrate how Rust can target everything from microcontrollers to iOS apps!

## Key Features

- **📝 Simple Content Creation**: Write presentations in Markdown or TOML format
- **🔄 Real-time Synchronization**: Multi-client synchronization via WebSocket protocol
- **🌐 Multi-platform Clients**: Web, Terminal, Desktop, iOS, and embedded support
- **🎯 Educational Focus**: Perfect for exploring Rust ecosystem


## Quick Start

### Install from source

```bash
# Clone the repository
git clone https://github.com/ilaborie/toboggan
cd toboggan

# Build all components
cargo build --release

# Run the server with example presentation
cargo run -p toboggan-server
```

### Create a presentation

```bash
# Convert Markdown to TOML
cargo run -p toboggan-cli -- examples/my-talk.md -o my-talk.toml

# Or create TOML directly
cat > my-talk.toml << 'EOF'
date = "2025-01-26"

[title]
type = "Text"
text = "My Presentation"

[[slides]]
kind = "Cover"
[slides.title]
type = "Text"
text = "Welcome"
EOF
```

### Serve and present

```bash
# Start the server
cargo run -p toboggan-server -- my-talk.toml

# Open web interface
open http://localhost:8080

# Or use terminal client
cargo run -p toboggan-tui
```

## Building

### Prerequisites

- Rust 1.83+ (2024 edition)
- Node.js 20+ (for web frontend)
- `mise` (optional, for task automation)

### Build all components

```bash
# Using mise (recommended)
mise check  # Format, lint, and test
mise build  # Build all components

# Or using cargo directly
cargo build --release
cargo test
cargo fmt
cargo clippy
```

### Platform-specific builds

#### Web (WASM)
```bash
mise build:wasm
# Or manually:
cd toboggan-web/toboggan-wasm
wasm-pack build --target web --release
```

#### iOS
```bash
mise build:ios
# Or manually:
cd toboggan-mobile
./build.sh
```

#### Desktop
```bash
cargo build -p toboggan-desktop --release
```

#### Terminal UI
```bash
cargo build -p toboggan-tui --release
```

## Architecture

Toboggan is designed as a modular system with clear separation of concerns. The architecture follows Clean Architecture principles with well-defined boundaries between components.

### Workspace Components

```
toboggan/
├── toboggan-core/       # Core domain models and business logic
├── toboggan-server/     # Axum server with WebSocket & REST
├── toboggan-client/     # Shared client library with WebSocket support
├── toboggan-cli/        # Command-line tool for Markdown → TOML conversion
├── toboggan-web/        # Web frontend with TypeScript and WASM client
├── toboggan-tui/        # Terminal UI client using ratatui
├── toboggan-desktop/    # Native desktop app using iced framework
├── toboggan-mobile/        # iOS Rust library with UniFFI bindings
├── TobogganApp/         # Native iOS app using SwiftUI
└── toboggan-esp32/      # ESP32 embedded client (excluded from workspace)
```

### Core Design Principles

- **WebSocket Protocol**: JSON-based real-time communication
- **Memory Safety**: Zero (direct) unsafe code, comprehensive error handling
- **Cross-platform**: Single codebase targeting multiple platforms
- **Modular Design**: Clear separation between server, clients, and core logic

## Client Applications

Toboggan supports multiple client types, each optimized for different use cases and platforms.

### Web Browser (`toboggan-web`)
- **Technology**: TypeScript frontend with WASM client
- **Features**: Modern web interface, keyboard shortcuts, responsive design
- **Usage**: Open `http://localhost:8080` when server is running
- **Platform**: Any modern web browser

### Terminal UI (`toboggan-tui`)
- **Technology**: [ratatui](https://ratatui.rs/) with crossterm
- **Features**: Full-featured terminal interface, presenter view, slide navigation
- **Usage**: `cargo run -p toboggan-tui`
- **Platform**: Linux, macOS, Windows terminals

### Desktop Application (`toboggan-desktop`)
- **Technology**: [iced](https://github.com/iced-rs/iced) native GUI framework
- **Features**: Native desktop experience with system integration
- **Usage**: `cargo run -p toboggan-desktop`
- **Platform**: Linux, macOS, Windows native

### iOS Application (`TobogganApp/`)
- **Technology**: SwiftUI with Rust core via UniFFI
- **Features**: Native iOS interface, gesture controls, AirPlay support
- **Usage**: Build and run from Xcode
- **Platform**: iOS 16+ devices and simulator

### Embedded Client (`toboggan-esp32`)
- **Technology**: ESP-IDF with embedded-graphics
- **Hardware**: ESP32-S3-BOX-3B development board
- **Features**: WiFi connectivity, LCD display, LED indicators
- **Platform**: ESP32 microcontrollers

## WebSocket Protocol

Toboggan uses a simple JSON-based WebSocket protocol for real-time synchronization:

### Commands (Client → Server)
- `Next`, `Previous`, `First`, `Last` - Navigation
- `Goto { slide: N }` - Jump to specific slide
- `Play`, `Pause`, `Resume` - Presentation control
- `Register { client_id }` - Client registration

### Notifications (Server → Clients)
- `State { current_slide, state }` - Presentation state updates
- `Error { message }` - Error notifications
- `Pong` - Heartbeat response

## Development

### Running tests
```bash
cargo test              # All tests
cargo nextest run      # Faster parallel tests
cargo test -p toboggan-core  # Specific crate
```

### Code quality
```bash
cargo fmt              # Format code
cargo clippy           # Lint code
mise check            # All checks
```

### Documentation
```bash
cargo doc --open      # Generate and open docs
```

## Contributing

We welcome contributions to Toboggan! Here's how you can help:

### Getting Started
1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes following the project guidelines
4. Run tests: `mise check` or `cargo test`
5. Submit a pull request

### Development Guidelines
- **Code Quality**: All code must pass `cargo fmt`, `cargo clippy`, and tests
- **Safety**: No `unsafe` code allowed (enforced by lints)
- **Error Handling**: Use `Result` and `Option`, avoid `unwrap()` in favor of `expect()`
- **Documentation**: Document public APIs and complex logic
- **Testing**: Add tests for new features and bug fixes

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Built with excellent Rust crates including:

**Core Infrastructure**
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime powering the server and clients
- [axum](https://github.com/tokio-rs/axum) - Web framework for the REST API and WebSocket server
- [serde](https://github.com/serde-rs/serde) - Serialization framework for all data structures
- [anyhow](https://github.com/dtolnay/anyhow) - Flexible error handling across the project

**Client Platforms**
- [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) - WebAssembly bindings for browser
- [web-sys](https://github.com/rustwasm/wasm-bindgen) - Browser API bindings for WASM
- [gloo](https://github.com/rustwasm/gloo) - Toolkit for building WASM applications
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [iced](https://github.com/iced-rs/iced) - Native desktop GUI framework
- [uniffi](https://github.com/mozilla/uniffi-rs) - Rust-Swift interoperability for iOS
- [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) - ESP-IDF services for ESP32
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) - 2D graphics for embedded displays
- [mipidsi](https://github.com/almindor/mipidsi) - MIPI Display Interface driver

**Networking & Communication**
- [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) - Async WebSocket implementation
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client for API calls
- [tower-http](https://github.com/tower-rs/tower-http) - HTTP middleware and services

**Utilities**
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [tracing](https://github.com/tokio-rs/tracing) - Structured application logging
- [jiff](https://github.com/BurntSushi/jiff) - Date and time handling
- [toml](https://github.com/toml-rs/toml) - TOML configuration parsing
- [comrak](https://github.com/kivikakk/comrak) - Markdown parsing and rendering

And many more amazing crates that make Rust development a joy!