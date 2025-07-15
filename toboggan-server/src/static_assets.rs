use rust_embed::RustEmbed;

/// Embedded web assets from toboggan-web/dist
#[derive(RustEmbed)]
#[folder = "../toboggan-web/dist"]
pub struct WebAssets;
