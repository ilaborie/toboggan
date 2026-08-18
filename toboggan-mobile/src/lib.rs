//! The mobile client core, exposed to Swift and Kotlin through [UniFFI].
//!
//! The connection, the protocol and the deck model live here; the platform app
//! only draws. Both the `SwiftUI` iOS app and the Kotlin Android app are hosts
//! for this crate.
//!
//! The `types` module mirrors [`toboggan_core`] rather than re-exporting it,
//! because
//! `UniFFI` needs its own record and enum definitions and the FFI surface is
//! allowed to be flatter than the domain model.
//!
//! Nothing here may panic: a panic unwinds into Objective-C or JNI and takes the
//! host app down, usually showing the user nothing.
//!
//! [UniFFI]: https://github.com/mozilla/uniffi-rs

uniffi::setup_scaffolding!("toboggan");

mod types;
pub use self::types::*;

mod handler;
pub use self::handler::*;

mod client;
pub use self::client::*;
