use std::fs;
use std::net::SocketAddr;
use std::path::Path;

mod settings;
use anyhow::Context;
use toboggan_core::Talk;
use tracing::info;
use tracing::instrument;

pub use self::settings::*;

mod state;
pub use self::state::*;

mod router;
pub use self::router::*;

#[instrument]
pub async fn launch(settings: Settings) -> anyhow::Result<()> {
    info!(?settings, "launching server...");
    let Settings { host, port, talk } = settings;

    let talk = load_talk(&talk).context("Loading talk")?;

    let addr = SocketAddr::from((host, port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Connecting to {addr} ..."))?;

    let state = TobogganState::new(talk);
    let router = routes().with_state(state);

    axum::serve(listener, router.into_make_service())
        .await
        .context("Axum server")?;

    Ok(())
}

#[instrument]
fn load_talk(path: &Path) -> anyhow::Result<Talk> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Reading talk file {}", path.display()))?;
    let result = toml::from_str(&content).context("Parsing talk")?;
    Ok(result)
}
