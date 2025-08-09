# toboggan-core

[![Crates.io](https://img.shields.io/crates/v/toboggan-core.svg)](https://crates.io/crates/toboggan-core)
[![Docs.rs](https://docs.rs/toboggan-core/badge.svg)](https://docs.rs/toboggan-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/yourusername/toboggan)

A `no_std` compatible presentation system for creating and managing slide-based talks.

## Overview

Toboggan Core provides the foundational types and abstractions for the Toboggan presentation ecosystem. It's designed to work across different environments: from embedded systems to WebAssembly browsers to traditional desktop applications.

## Features

- **`no_std` compatible** - Works in constrained environments
- **Multi-platform** - Supports embedded, WASM, and standard library environments
- **Rich content types** - Text, HTML, iframes, terminals, and layout containers
- **Type-safe** - Extensive use of Rust's type system for correctness
- **Serializable** - All types can be serialized with serde
- **Well-documented** - Comprehensive API documentation with examples

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
toboggan-core = "0.1.0"
```

Create a simple presentation:

```rust
use toboggan_core::{Talk, Slide, Content, Date};

let talk = Talk::new("My Presentation")
    .with_date(Date::new(2025, 1, 26))
    .add_slide(
        Slide::cover("Welcome")
            .with_body("Welcome to my presentation")
    )
    .add_slide(
        Slide::new("Introduction")
            .with_body(Content::html("<p>This is <strong>HTML</strong> content</p>"))
            .with_notes("Remember to speak slowly")
    );
```

## Feature Flags

### Standard Features
- **`std`** (default): Full standard library support
  - File system access for loading content from files
  - Terminal slides for live demonstrations
  - Full UUID generation for client IDs

### No-std Features  
- **`alloc`**: Heap allocation support for `no_std` environments
  - Required for dynamic collections and most functionality
  - Automatically enabled by `std` feature

### Platform-specific Features
- **`js`**: WebAssembly support with JavaScript bindings
  - System time access via JavaScript APIs
  - Secure random number generation in browsers

### Optional Features
- **`openapi`**: Enables OpenAPI schema generation
- **`test-utils`**: Testing utilities for development

## Multi-platform Usage

### Standard Library (Desktop, Server)
```toml
[dependencies]
toboggan-core = { version = "0.1.0", features = ["std"] }
```

### WebAssembly (Browser)
```toml
[dependencies]
toboggan-core = { version = "0.1.0", default-features = false, features = ["alloc", "js"] }
```

### Embedded Systems (no_std with alloc)
```toml
[dependencies]
toboggan-core = { version = "0.1.0", default-features = false, features = ["alloc"] }
```

## Content Types

Toboggan supports rich content through the [`Content`] enum:

```rust
use toboggan_core::Content;

// Simple text
let text = Content::from("Plain text");

// HTML with accessibility
let html = Content::html_with_alt(
    "<img src='chart.png'>",
    "Sales chart showing growth"
);

// Layout containers
let layout = Content::hbox("1fr 1fr", [
    Content::from("Left column"),
    Content::html("<div>Right column</div>")
]);
```

### File-based Content (std only)

Load content directly from files with automatic type detection:

```rust
use std::path::Path;
use toboggan_core::Content;

// Automatically converts markdown to HTML
let content = Content::from(Path::new("slides/intro.md"));

// Uses HTML directly  
let html = Content::from(Path::new("slides/chart.html"));

// Treats as plain text
let text = Content::from(Path::new("notes.txt"));
```

## Presentation State

Manage presentation state with built-in support for timing and navigation:

```rust
use toboggan_core::{State, SlideId};
use std::time::Duration;

let slide1 = SlideId::next();
let mut state = State::Paused {
    current: slide1,
    total_duration: Duration::ZERO,
};

// Resume presentation
state.auto_resume();
// Now state is State::Running with timing information
```

## Architecture

### Memory Safety
- **No `unsafe` code** - Enforced by workspace lints
- **Comprehensive error handling** - Uses `Result` and `Option` appropriately
- **Descriptive error messages** - Avoids `unwrap()` in favor of `expect()`

### Performance
- **Zero-cost abstractions** - Efficient code generation
- **Atomic operations** - Thread-safe ID generation
- **Efficient serialization** - Optimized for network transmission

### Compatibility
- **Rust 2024 edition** - Uses latest language features
- **WASM-compatible** - Works in `wasm32-unknown-unknown` target
- **Thread-safe** - Safe to use in multi-threaded environments

## Examples

See the [`examples/`](../examples/) directory for complete examples of using toboggan-core in different environments.

## Contributing

Contributions are welcome! Please see the [contributing guidelines](../CONTRIBUTING.md) for details.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.