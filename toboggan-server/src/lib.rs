//! The Toboggan server: an axum service that serves a deck over REST, keeps
//! every client on the same slide over a WebSocket, and hosts the embedded web
//! client.
//!
//! [`launch_with_talk`] is the serving core; [`launch`] wraps it for a talk read
//! from a file. [`routes`] and [`routes_with_cors`] expose the router for
//! mounting elsewhere.
//!
//! Navigation and the embedded terminals are gated on the presenter role — see
//! [`PresenterAuth`], and `SECURITY.md` at the repository root, because
//! `/api/terminal` spawns a real shell.
//!
//! The crate embeds `toboggan-web/dist` at compile time; its `build.rs` fails
//! when that folder is absent.

mod auth;
pub use self::auth::PresenterAuth;

mod settings;
pub use self::settings::*;

mod dto;
pub use self::dto::*;

mod services;
pub use self::services::*;

mod state;
pub use self::state::*;

mod router;
pub use self::router::{routes, routes_for_shots, routes_with_cors};

mod watcher;
pub use self::watcher::*;

mod bootstrap;
pub use self::bootstrap::{
    EphemeralServer, launch, launch_with_talk, openapi_json, serve_ephemeral,
};
