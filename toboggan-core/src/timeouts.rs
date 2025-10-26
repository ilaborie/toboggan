//! Shared timeout and interval constants for Toboggan
//!
//! This module provides centralized configuration for all timing-related
//! constants used across the client, server, and other components.

#[cfg(all(not(feature = "std"), feature = "js"))]
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::Duration;

/// Interval between server heartbeat pings to clients
///
/// The server sends periodic pings to keep the WebSocket connection alive
/// and detect disconnected clients.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Interval between client pings to server
///
/// Clients send pings to measure round-trip time and keep the connection alive.
pub const PING_PERIOD: Duration = Duration::from_secs(10);

/// Maximum time to wait for a connection response
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

/// Interval for cleanup tasks
///
/// How often the server checks for and removes disconnected clients.
pub const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);
