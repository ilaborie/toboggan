//! Toboggan client library for API and WebSocket communication.
//!
//! This crate provides a shared client implementation for connecting to
//! Toboggan servers, including REST API access and real-time WebSocket
//! communication with automatic reconnection.

mod api;
pub use self::api::*;

mod communication;
pub use self::communication::*;

mod config;
pub use self::config::*;
