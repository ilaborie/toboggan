use anyhow::{Context, Result};
use clap::Parser;
use toboggan_client::TobogganConfig;
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

    // Run the app
    let terminal = ratatui::init();
    let result = {
        App::new(terminal, &config)
            .await
            .context("create app")
            .and_then(|mut app| app.run())
    };
    ratatui::restore();

    result
}
