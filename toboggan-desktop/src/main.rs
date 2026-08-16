use anyhow::Result;
use toboggan_client::TobogganConfig;
use toboggan_desktop::run;

fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    run(TobogganConfig::default())
}
