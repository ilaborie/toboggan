use axum::body::Bytes;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
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

/// Builds a `200 OK` asset response, guessing the content type from `path`.
/// Shared by every handler that serves bytes (embedded web/guide assets and the
/// generated slide-overview cache).
pub(super) fn asset_response(path: &str, bytes: impl Into<Bytes>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime.as_ref().to_owned())],
        bytes.into(),
    )
        .into_response()
}

/// How long the browser may keep a file from the embedded web bundle.
///
/// Every load of `/run` pulls down the wasm module and the terminal font — over
/// a megabyte — and none of it was cacheable, so it came down the wire again on
/// every reload, in front of the first slide.
fn bundle_cache_control(path: &str) -> Option<&'static str> {
    if path.starts_with("assets/") {
        // Vite content-hashes these, so a changed file is a changed URL and
        // there is nothing to revalidate.
        Some("public, max-age=31536000, immutable")
    } else if path.starts_with("fonts/") {
        // Stable filenames, so this one is allowed to go stale: cache it for a
        // week rather than pinning a swapped font for a year.
        Some("public, max-age=604800")
    } else {
        // `index.html`, the icons and `manifest.json` — the entry points that
        // must be free to change on the next release.
        None
    }
}

/// [`asset_response`] plus the cache policy for the embedded web bundle.
pub(super) fn bundle_asset_response(path: &str, bytes: impl Into<Bytes>) -> Response {
    let mut response = asset_response(path, bytes);
    if let Some(cache_control) = bundle_cache_control(path) {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_content_hashed_files_are_cached_forever() {
        assert_eq!(
            bundle_cache_control("assets/toboggan_wasm_bg-lsTrCMZy.wasm"),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(
            bundle_cache_control("fonts/JetBrainsMonoNerdFontMono-Regular.woff2"),
            Some("public, max-age=604800")
        );
    }

    /// The entry points carry the references to every hashed file, so pinning
    /// them would pin the whole bundle and a release would never be picked up.
    #[test]
    fn the_entry_points_are_never_pinned() {
        for path in [
            "index.html",
            "presenter.html",
            "manifest.json",
            "favicon.ico",
        ] {
            assert_eq!(bundle_cache_control(path), None, "{path} must stay fresh");
        }
    }
}
