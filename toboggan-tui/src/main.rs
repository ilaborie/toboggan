use anyhow::{Context, Result};
use clap::Parser;
use toboggan_tui::{App, build_config};
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(name = "toboggan-tui")]
#[command(about = "Terminal-based Toboggan presentation client")]
struct Cli {
    #[arg(short, long, default_value = "ws://localhost:8080/api/ws")]
    websocket_url: String,

    #[arg(short, long, default_value = "http://localhost:8080")]
    api_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup tui-logger
    tracing_subscriber::registry()
        .with(tui_logger::TuiTracingSubscriberLayer)
        .init();
    tui_logger::init_logger(tui_logger::LevelFilter::Debug).context("init tui_logger")?;

    // Create config using toboggan-client shared config
    let config = build_config(Some(cli.websocket_url), Some(cli.api_url));

    // Run the app
    let terminal = ratatui::init();
    let result = App::new(terminal, &config)
        .await
        .context("create app")
        .and_then(|mut app| app.run());
    ratatui::restore();

    result
}
