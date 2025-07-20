use anyhow::Context;
use clap::Parser;
use toboggan_server::{Settings, launch};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::try_parse().context("parsing arguments")?;

    // Validate settings before initializing logging
    if let Err(msg) = settings.validate() {
        return Err(anyhow::anyhow!("validating settings: {msg}"));
    }

    // Initialize logging with configured level
    let log_level = settings.log_level.parse().unwrap_or(tracing::Level::INFO);

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .pretty()
        .init();

    launch(settings).await?;

    Ok(())
}
