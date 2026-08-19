//! The domain model for Toboggan, and the protocol its clients speak.
//!
//! This crate depends on nothing else in the workspace; everything else is
//! built on top of it. The overview below is the crate's README, so the
//! examples in it are doctests and cannot drift from the API they describe.
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod time;
pub use self::time::*;

mod config;
pub use self::config::*;

pub mod timeouts;

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

mod terminal;
pub use self::terminal::*;

mod client;
pub use self::client::*;

mod secret;
pub use self::secret::*;

mod goto;
pub use self::goto::*;
