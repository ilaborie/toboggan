use anyhow::{Context, Result};
use clap::Parser;
use toboggan_client::{TobogganApi, TobogganConfig};
use toboggan_core::ClientConfig;
use toboggan_tui::App;
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(name = "toboggan-tui")]
#[command(about = "Terminal-based Toboggan presentation client")]
struct Cli {
    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Cli { host, port } = Cli::parse();

    // Setup tui-logger
    tracing_subscriber::registry()
        .with(tui_logger::TuiTracingSubscriberLayer)
        .init();
    tui_logger::init_logger(tui_logger::LevelFilter::Debug).context("init tui_logger")?;

    // Create config using toboggan-client shared config
    let config = TobogganConfig::new(&host, port);

    // Fetch data async before terminal init
    let api = TobogganApi::new(config.api_url());
    let talk = api.talk().await.context("fetching talk")?;
    let slides = api.slides().await.context("fetching slides")?;

    // Run the app with ratatui::run() for clean terminal management
    ratatui::run(|terminal| App::new(&config, api, talk, slides.slides).run(terminal))
}
