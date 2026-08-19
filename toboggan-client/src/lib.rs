//! The shared client library: the socket, the reconnection, and the dispatch.
//!
//! Used by the terminal, desktop and mobile clients, which differ only in what
//! they draw. [`TobogganApi`] is the REST half, [`WebSocketClient`] the live
//! half, and [`TobogganClientCore`] runs the two together against a
//! [`NotificationHandler`].
//!
//! Reconnection backs off exponentially with jitter — without the jitter, every
//! client in a room that lost its wifi comes back at the same instant.

mod api;
pub use self::api::*;

mod client;
pub use self::client::*;

mod communication;
pub use self::communication::*;

mod config;
pub use self::config::*;

mod notification;
pub use self::notification::*;
