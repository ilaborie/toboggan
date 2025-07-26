use anyhow::Result;
use clap::Parser;
use iced::{Application, Settings, Size};
use tracing::info;

mod app;
mod config;
mod messages;
mod services;
mod ui;

use app::TobogganApp;
use config::Config;

#[derive(Parser)]
#[command(name = "toboggan-desktop")]
#[command(about = "Toboggan Desktop Presentation Client")]
struct Args {
    #[arg(short, long, help = "WebSocket server URL")]
    url: Option<String>,

    #[arg(short, long, help = "Configuration file path")]
    config: Option<String>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let config = Config::load(args.config.as_deref(), args.url)?;
    info!("Starting Toboggan Desktop with config: {:?}", config);

    let settings = Settings {
        window: iced::window::Settings {
            size: Size::new(1024.0, 768.0),
            position: iced::window::Position::Centered,
            ..Default::default()
        },
        ..Default::default()
    };

    TobogganApp::run(Settings {
        flags: config,
        ..settings
    })
    .map_err(|error| anyhow::anyhow!("Application error: {}", error))
}
