use anyhow::Result;
use clap::Parser;
use toboggan_client::TobogganConfig;
use toboggan_tui::{init_tui_logger, run};

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

    init_tui_logger()?;

    let config = TobogganConfig::new(&host, port);
    run(&config).await
}
