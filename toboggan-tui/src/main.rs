use anyhow::Result;
use clap::Parser;
use toboggan_tui::app::App;
use toboggan_tui::config::Config;

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

    // Setup tracing
    tracing_subscriber::fmt::init();

    // Create config
    let config = Config {
        websocket_url: cli.websocket_url,
        api_url: cli.api_url,
        max_retries: 5,
        retry_delay_ms: 1000,
    };

    // Run the app
    let mut app = App::new(config)?;
    app.run().await
}
