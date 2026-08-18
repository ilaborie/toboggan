//! The terminal client: a [ratatui] view of the deck, with speaker notes, a
//! next-slide preview and a slide list.
//!
//! Connects to a running server and stays in sync with every other client. Run
//! it with `toboggan tui`; this crate declares no binary of its own.
//!
//! [ratatui]: https://ratatui.rs/

#![allow(clippy::missing_errors_doc)]

use anyhow::Context;
use toboggan_client::{TobogganApi, TobogganConfig};
use toboggan_core::ClientConfig;
use tracing_subscriber::prelude::*;

mod app;
pub use self::app::*;

pub(crate) mod connection_handler;
pub(crate) mod effects;
pub(crate) mod events;
pub(crate) mod state;
pub(crate) mod ui;

/// Installs the `tui_logger` tracing subscriber.
///
/// Must be called before [`run`]; a plain stdout logger would corrupt the
/// terminal UI. Shared by the standalone `toboggan-tui` binary and the unified
/// `toboggan tui` subcommand.
pub fn init_tui_logger() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tui_logger::TuiTracingSubscriberLayer)
        .init();
    tui_logger::init_logger(tui_logger::LevelFilter::Debug).context("init tui_logger")?;
    Ok(())
}

/// Fetches the talk from a running server and runs the terminal UI.
pub async fn run(config: &TobogganConfig) -> anyhow::Result<()> {
    let api = TobogganApi::new(config.api_url());
    let talk = api.talk().await.context("fetching talk")?;
    let slides = api.slides().await.context("fetching slides")?;

    // Use ratatui::run() for clean terminal management
    ratatui::run(|terminal| App::new(config, api, talk, slides.slides).run(terminal))
}
