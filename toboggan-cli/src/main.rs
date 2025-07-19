use anyhow::Context;
use clap::Parser;
use toboggan_cli::{Settings, launch};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().pretty().init();

    let settings = Settings::try_parse().context("parsing arguments")?;

    launch(settings)?;

    Ok(())
}
