use toboggan_server::{WatchConfig, WatchTarget};
use tracing::info;

use super::deck::{build_talk, build_talk_lossy, resolve_deck};
use crate::cli::DefaultArgs;

/// Builds the folder in-memory and serves it, optionally watching for changes.
pub(crate) async fn build_and_serve(
    args: DefaultArgs,
    config: crate::config::Config,
) -> anyhow::Result<()> {
    let crate::cli::ResolvedDefault {
        input,
        cli: mut cli_settings,
        server: mut server_settings,
        watch,
    } = args.resolve(config)?;

    // Resolve the slides folder and a sibling `public/` when the deck root is given.
    let deck = resolve_deck(&input);
    let slides_dir = deck.slides;
    cli_settings.input = Some(slides_dir.clone());
    if server_settings.public_dir.is_none() {
        server_settings.public_dir = deck.public;
    }

    let talk = build_talk(&slides_dir, &cli_settings)?;

    let watch_config = watch.then(|| {
        let folder = slides_dir.clone();
        let settings = cli_settings.clone();
        // Watch the assets directory alongside the slides: editing a stylesheet
        // or an image leaves the `Talk` identical, but the reload still notifies
        // clients, and re-rendering re-fetches whatever the slide references.
        //
        // Only if it exists: `--public-dir` may point at a folder the author has
        // not created yet, and the watcher errors on a missing path — serving
        // without live-reloaded assets beats refusing to start.
        let assets = server_settings
            .public_dir
            .clone()
            .filter(|dir| dir.is_dir());
        WatchConfig {
            target: WatchTarget::Deck {
                slides: slides_dir.clone(),
                assets,
            },
            // Lossy on reload only: a half-written slide mid-rehearsal should
            // not tear down the running server, but it is logged rather than
            // dropped in silence.
            reload: Box::new(move || build_talk_lossy(&folder, &settings)),
        }
    });

    info!(slides = %slides_dir.display(), watch, "build + serve");
    // The served deck's diagrams must be drawn the same way whether they come
    // out of `/run` or out of `/download.pdf`, so the renderer travels with the
    // server rather than being rebuilt from defaults inside it.
    let mermaid = toboggan_cli::mermaid_renderer(&cli_settings)?;
    toboggan_server::launch_with_talk(talk, server_settings, watch_config, mermaid).await
}
