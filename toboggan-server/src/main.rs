use anyhow::Context;
use clap::Parser;
use toboggan_server::{Settings, launch};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::try_parse().context("parsing arguments")?;

    // Validate settings before initializing logging
    if let Err(msg) = settings.validate() {
        return Err(anyhow::anyhow!("validating settings: {msg}"));
    }

    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    launch(settings).await?;

    Ok(())
}
