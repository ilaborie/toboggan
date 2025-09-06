# toboggan-core

[![Crates.io](https://img.shields.io/crates/v/toboggan-core.svg)](https://crates.io/crates/toboggan-core) [![Docs.rs](https://docs.rs/toboggan-core/badge.svg)](https://docs.rs/toboggan-core) [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/yourusername/toboggan)

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
use toboggan_core::{Talk, Slide, Content, Date, SlideKind};

// Create a new presentation
let mut talk = Talk::builder()
    .title("My Rust Conference Talk")
    .date(Date::new(2025, 1, 26))
    .build();

// Add a cover slide
let cover_slide = Slide::builder()
    .kind(SlideKind::Cover)
    .title("Welcome to Rust")
    .body(Content::from("An introduction to systems programming"))
    .notes("Start with energy, make eye contact")
    .build();

// Add content slides
let intro_slide = Slide::builder()
    .title("Why Rust?")
    .body(Content::html_with_alt(
        r#"<ul>
            <li>Memory safety without garbage collection</li>
            <li>Zero-cost abstractions</li>
            <li>Fearless concurrency</li>
        </ul>"#,
        "Key benefits of Rust: memory safety, zero-cost abstractions, fearless concurrency"
    ))
    .notes("Emphasize the unique value proposition")
    .build();

// Add slides to presentation
talk.add_slide(cover_slide);
talk.add_slide(intro_slide);

// Presentation is ready to serialize or serve
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

Toboggan supports rich content through the [`Content`] enum, designed for accessibility and multi-platform rendering:

```rust
use toboggan_core::Content;

// Simple text content
let text = Content::from("Plain text content");

// HTML with accessibility fallback
let html = Content::html_with_alt(
    r#"<div class="chart">
        <img src="growth.png" alt="Sales chart">
        <p>Q4 growth: +25%</p>
    </div>"#,
    "Sales chart showing 25% growth in Q4"
);

// Markdown content (converted to HTML)
let markdown = Content::markdown_with_alt(
    r#"## Key Points

- **Performance**: Zero-cost abstractions
- **Safety**: Memory safety without GC
- **Concurrency**: Fearless parallelism"#,
    "Three key points about Rust performance, safety, and concurrency"
);

// Layout containers for complex layouts
let two_column = Content::hbox("1fr 1fr", vec![
    Content::from("Left column content"),
    Content::html("<strong>Right column</strong>")
]);

let three_row = Content::vbox("auto 1fr auto", vec![
    Content::from("Header"),
    Content::html("<main>Main content area</main>"),
    Content::from("Footer")
]);

// Nested layouts for complex designs
let complex_layout = Content::hbox("2fr 1fr", vec![
    Content::vbox("auto 1fr", vec![
        Content::from("Main content title"),
        Content::markdown("## Details\n\nDetailed information here")
    ]),
    Content::vbox("1fr", vec![
        Content::html("<aside>Sidebar content</aside>")
    ])
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

## API Reference

### Core Types

The toboggan-core crate provides several key types for building presentations:

#### `Talk` - Presentation Container

```rust
use toboggan_core::{Talk, Date};

// Create a new presentation
let talk = Talk::builder()
    .title("My Conference Talk")
    .date(Date::new(2024, 12, 31))
    .build();

// Or create with constructor
let talk = Talk::new("Simple Talk");
```

#### `Slide` - Individual Presentation Slide

```rust
use toboggan_core::{Slide, SlideKind, Content};

// Different slide types
let cover = Slide::builder()
    .kind(SlideKind::Cover)
    .title("Welcome")
    .build();

let content_slide = Slide::builder()
    .kind(SlideKind::Default)
    .title("Main Topic")
    .body(Content::from("Slide content here"))
    .notes("Speaker notes")
    .build();

let section = Slide::builder()
    .kind(SlideKind::Part)
    .title("Section 2")
    .build();
```

#### `Content` - Rich Content System

```rust
use toboggan_core::Content;

// Text content
Content::from("Simple text");

// HTML with accessibility
Content::html_with_alt(
    "<p>Rich HTML</p>",
    "Accessible description"
);

// Layout containers
Content::hbox("1fr 2fr", vec![/* content */]);
Content::vbox("auto 1fr auto", vec![/* content */]);
```

#### `State` - Presentation Runtime State

```rust
use toboggan_core::{State, SlideId};

// State management for presentation control
match state {
    State::Init => { /* Initial state */ },
    State::Paused { current, .. } => { /* Presentation paused */ },
    State::Running { current, started, .. } => { /* Active presentation */ },
    State::Done { current, .. } => { /* Presentation finished */ },
}
```

### Serialization Support

All core types support serde serialization for network transport and storage:

```rust
use serde_json;

// Serialize presentation to JSON
let json = serde_json::to_string(&talk)?;

// Deserialize from JSON
let talk: Talk = serde_json::from_str(&json)?;

// Also supports TOML, MessagePack, etc.
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

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
