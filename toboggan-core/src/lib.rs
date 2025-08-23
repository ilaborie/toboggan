//! # Toboggan Core
//!
//! A `no_std` compatible presentation system for creating and managing slide-based talks.
//! This crate provides the core domain models and types used across the Toboggan ecosystem.
//!
//! ## Overview
//!
//! Toboggan Core is designed to work in constrained environments, supporting both standard
//! library environments and `no_std` environments with heap allocation. It can even run
//! in WebAssembly browsers and embedded systems.
//!
//! ## Key Components
//!
//! - **[`Talk`]**: Main presentation container with title, date, and slides
//! - **[`Slide`]**: Individual slide with kind, style, title, body, and notes
//! - **[`Content`]**: Rich content types (Text, Html, `IFrame`, Terminal, Layout containers)
//! - **[`State`]**: Presentation state management (Paused, Running, Done)
//! - **[`Command`]**: Actions that can be performed on presentations
//! - **[`Notification`]**: Events broadcast to connected clients
//! - **[`SlideId`]**: Unique identifiers for slides
//! - **[`Timestamp`]** and **[`Date`]**: Time handling type aliases for jiff types
//!
//! ## Feature Flags
//!
//! This crate supports multiple feature configurations for different environments:
//!
//! ### Standard Features
//! - **`std`** (default): Full standard library support
//!   - Enables all functionality including file system access
//!   - Provides `Content::Term` for terminal slides
//!   - Uses system time and full UUID generation
//!
//! ### No-std Features  
//! - **`alloc`**: Heap allocation support for `no_std` environments
//!   - Enables dynamic collections (Vec, String)
//!   - Required for most functionality in `no_std`
//!   - Automatically enabled by `std` feature
//!
//! ### Platform-specific Features
//! - **`js`**: WebAssembly support with JavaScript bindings
//!   - Enables `jiff/js` for system time access via JavaScript
//!   - Enables `getrandom/js` for secure random number generation
//!   - Use this when targeting WASM in browsers
//!
//! ### Optional Features
//! - **`openapi`**: Enables `OpenAPI` schema generation via `utoipa`
//! - **`test-utils`**: Provides testing utilities like `SlideId::reset_sequence()`
//!
//! ## Usage Examples
//!
//! ### Creating a Basic Talk
//!
//! ```rust
//! use toboggan_core::{Talk, Slide, Content, Date};
//!
//! let talk = Talk::new("My Presentation")
//!     .with_date(Date::ymd(2025, 1, 26))
//!     .add_slide(
//!         Slide::cover("Welcome")
//!             .with_body("Welcome to my presentation")
//!     )
//!     .add_slide(
//!         Slide::new("Introduction")
//!             .with_body(Content::html("<p>This is <strong>HTML</strong> content</p>"))
//!             .with_notes("Remember to speak slowly")
//!     );
//! ```
//!
//! ### Working with Content Types
//!
//! ```rust
//! use toboggan_core::Content;
//!
//! // Simple text content
//! let text = Content::from("Plain text");
//!
//! // HTML content
//! let html = Content::html("<h1>Title</h1>");
//!
//! // HTML content
//! let image = Content::html("<img src='chart.png' alt='Chart showing 50% increase in sales'>");
//! ```
//!
//!
//! ### Presentation State Management
//!
//! ```rust
//! use toboggan_core::{State, SlideId, Command, Duration};
//!
//! let slide1 = SlideId::next();
//! let mut state = State::Paused {
//!     current: Some(slide1),
//!     total_duration: Duration::ZERO,
//! };
//!
//! // Resume presentation
//! state.auto_resume();
//! // Now state is State::Running { since: Timestamp::now(), current: slide1, total_duration: Duration::ZERO }
//! ```
//!
//! ## Multi-platform Support
//!
//! ### Standard Library (Desktop, Server)
//! ```toml
//! [dependencies]
//! toboggan-core = { version = "0.1.0", features = ["std"] }
//! ```
//!
//! ### WebAssembly (Browser)
//! ```toml
//! [dependencies]
//! toboggan-core = { version = "0.1.0", default-features = false, features = ["alloc", "js"] }
//! ```
//!
//! ### Embedded Systems (`no_std` with alloc)
//! ```toml
//! [dependencies]
//! toboggan-core = { version = "0.1.0", default-features = false, features = ["alloc"] }
//! ```
//!
//! ### Minimal Embedded (`no_std`, no alloc)
//! ```toml
//! [dependencies]
//! toboggan-core = { version = "0.1.0", default-features = false }
//! ```
//! Note: Some functionality is limited without `alloc` (e.g., dynamic slide collections).
//!
//! ## Architecture Notes
//!
//! ### Memory Safety
//! - No `unsafe` code - enforced by `#![forbid(unsafe_code)]` workspace lint
//! - Extensive use of `Result` and `Option` for error handling
//! - Avoids `unwrap()` in favor of `expect()` with descriptive messages
//!
//! ### Performance
//! - Zero-cost abstractions where possible
//! - Efficient serialization with `serde`
//! - Atomic operations for thread-safe ID generation
//! - Memory leak prevention in WASM environments
//!
//! ### Compatibility
//! - Supports Rust 2024 edition
//! - Compatible with `wasm32-unknown-unknown` target
//! - Works in single-threaded and multi-threaded environments
//! - Comprehensive test coverage across all feature combinations

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod time;
pub use self::time::*;

mod slide_id;
pub use self::slide_id::*;

mod content;
pub use self::content::*;

mod state;
pub use self::state::*;

mod slide;
pub use self::slide::*;

mod talk;
pub use self::talk::*;

mod command;
pub use self::command::*;

mod notification;
pub use self::notification::*;
