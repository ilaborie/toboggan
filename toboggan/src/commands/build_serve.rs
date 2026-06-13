use std::path::{Path, PathBuf};

use toboggan_core::Talk;
use toboggan_server::WatchConfig;
use tracing::info;

use crate::cli::DefaultArgs;

/// Builds the folder in-memory and serves it, optionally watching for changes.
pub(crate) async fn build_and_serve(args: DefaultArgs) -> anyhow::Result<()> {
    let (input, mut cli_settings, mut server_settings, watch) = args.resolve()?;

    // Resolve the slides folder and a sibling `public/` when the deck root is given.
    let (slides_dir, public_dir) = resolve_deck(&input);
    cli_settings.input = Some(slides_dir.clone());
    if server_settings.public_dir.is_none() {
        server_settings.public_dir = public_dir;
    }

    let talk = build_talk(&slides_dir, &cli_settings)?;

    let watch_config = watch.then(|| {
        let folder = slides_dir.clone();
        let settings = cli_settings.clone();
        WatchConfig {
            path: slides_dir.clone(),
            recursive: true,
            reload: Box::new(move || build_talk(&folder, &settings)),
        }
    });

    info!(slides = %slides_dir.display(), watch, "build + serve");
    toboggan_server::launch_with_talk(talk, server_settings, watch_config).await
}

/// Resolves the slides folder and an optional sibling `public/` directory.
///
/// Accepts either the slides folder directly, or a deck root containing a
/// `slides/` subdirectory (with `public/` alongside it).
fn resolve_deck(input: &Path) -> (PathBuf, Option<PathBuf>) {
    let nested_slides = input.join("slides");
    if nested_slides.is_dir() {
        let public = input.join("public");
        let public = public.is_dir().then_some(public);
        (nested_slides, public)
    } else {
        let public = input.parent().map(|parent| parent.join("public"));
        let public = public.filter(|path| path.is_dir());
        (input.to_path_buf(), public)
    }
}

fn build_talk(input: &Path, settings: &toboggan_cli::Settings) -> anyhow::Result<Talk> {
    let parse_result = toboggan_cli::parse_presentation(input, settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(parse_result.to_talk())
}
