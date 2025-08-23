//! Toboggan client library for API and WebSocket communication.

// Compile-time error if neither sync nor async features are enabled
#[cfg(not(any(feature = "sync", feature = "async")))]
compile_error!("Either 'sync' or 'async' feature must be enabled for toboggan-client");

#[cfg(feature = "async")]
mod r#async;
#[cfg(feature = "async")]
pub use self::r#async::*;

#[cfg(feature = "sync")]
mod sync;
#[cfg(feature = "sync")]
pub use self::sync::*;

mod config;
pub use self::config::*;
