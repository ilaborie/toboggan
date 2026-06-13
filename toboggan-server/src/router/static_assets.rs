use rust_embed::RustEmbed;

/// Embedded web assets from toboggan-web/dist
#[derive(RustEmbed)]
#[folder = "../toboggan-web/dist"]
pub(super) struct WebAppAssets;

/// Embedded `public/` assets (CSS, fonts, images) for the packaged guide,
/// served at `/guide/public/`.
#[derive(RustEmbed)]
#[folder = "../examples/toboggan-guide/public"]
pub(super) struct GuideAssets;
