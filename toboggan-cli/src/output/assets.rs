//! Rewriting a deck's asset URLs for the static HTML export.
//!
//! A deck keeps its assets in `public/`, beside the slides folder, and the
//! server maps `/public/` to it. Slides render at `/run`, so all three of
//! `../public/logo.png`, `/public/logo.png` and `./public/logo.png` reach the
//! same file while the deck is being served.
//!
//! The export has no server. It is one file dropped somewhere — `dist/index.html`
//! for the GitHub Action — and only the *relative* spelling resolves there:
//! `../public/…` points above the file's own directory, and `/public/…` points
//! at the site root, which is wrong the moment the site lives under a sub-path
//! like `user.github.io/repo/`.
//!
//! So every spelling is normalised to one: `{base}public/…`, where `base` is
//! whatever [`crate::Settings::base_url`] says the export will be served from,
//! and empty by default — a plain relative URL, which is correct wherever the
//! file ends up as long as `public/` travels with it.

use std::fmt::Write as _;

use crate::PUBLIC_DIR;

/// Attributes that name something the browser will fetch.
const URL_ATTRIBUTES: [&str; 2] = ["src=\"", "href=\""];

/// Rewrites every deck asset URL in `html` to sit under `base`.
///
/// Applies to the whole exported document, not just the markdown-rendered part:
/// a deck's `_head.html` links its stylesheet the same way its slides embed an
/// image, and an author writing `<img src="…">` by hand — which the guide
/// recommends for generated diagrams — never goes through the markdown renderer
/// at all.
pub(super) fn rewrite_asset_urls(html: &str, base: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some((prefix, attribute, tail)) = next_url_attribute(rest) {
        out.push_str(prefix);
        out.push_str(attribute);
        let Some((url, after)) = tail.split_once('"') else {
            // An unterminated attribute is not ours to repair.
            out.push_str(tail);
            return out;
        };
        match public_path(url) {
            Some(path) => {
                let _ = write!(out, "{base}{PUBLIC_DIR}/{path}");
            }
            None => out.push_str(url),
        }
        out.push('"');
        rest = after;
    }

    out.push_str(rest);
    out
}

/// Finds the next `src="` / `href="`, returning what came before it, the
/// attribute itself, and everything after the opening quote.
fn next_url_attribute(haystack: &str) -> Option<(&str, &'static str, &str)> {
    URL_ATTRIBUTES
        .iter()
        .filter_map(|attribute| haystack.find(attribute).map(|at| (at, *attribute)))
        .min_by_key(|(at, _)| *at)
        .map(|(at, attribute)| {
            let (prefix, found) = haystack.split_at(at);
            (prefix, attribute, &found[attribute.len()..])
        })
}

/// The part of `url` below `public/`, if it points there at all.
///
/// `None` for anything that is not the deck's own asset: an absolute URL, an
/// in-page anchor, a `data:` payload — and also for a relative URL that does not
/// mention `public/`, which may well be a link the author means to keep, such as
/// a sibling page or a server route.
fn public_path(url: &str) -> Option<String> {
    if url.is_empty()
        || url.starts_with('#')
        || url.starts_with("//")
        || url.contains("://")
        || ["data:", "mailto:", "tel:"]
            .iter()
            .any(|scheme| url.starts_with(scheme))
    {
        return None;
    }

    let (path, suffix) = split_query(url);
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            // A leading `/`, a `//` in the middle, and `./` all contribute
            // nothing; `..` climbs, clamped at the root because the export has
            // no notion of anywhere above itself.
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }

    let (first, rest) = segments.split_first()?;
    if *first != PUBLIC_DIR || rest.is_empty() {
        return None;
    }
    Some(format!("{}{suffix}", rest.join("/")))
}

/// Splits a URL into its path and everything from the first `?` or `#` on,
/// which is carried through untouched.
fn split_query(url: &str) -> (&str, &str) {
    let at = url.find(['?', '#']).unwrap_or(url.len());
    url.split_at(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite(html: &str) -> String {
        rewrite_asset_urls(html, "")
    }

    /// The three spellings that all reach one file on the server have to reach
    /// one file in the export too.
    #[test]
    fn every_spelling_of_an_asset_lands_in_the_same_place() {
        for spelling in [
            "../public/logo.png",
            "/public/logo.png",
            "./public/logo.png",
            "public/logo.png",
            "../../public/logo.png",
        ] {
            assert_eq!(
                rewrite(&format!(r#"<img src="{spelling}">"#)),
                r#"<img src="public/logo.png">"#,
                "{spelling}"
            );
        }
    }

    /// The markdown renderer is not the only thing that emits a URL: a deck's
    /// `_head.html` links its stylesheet, and the guide tells authors to embed
    /// generated diagrams with a hand-written `<img>`.
    #[test]
    fn head_links_are_rewritten_too() {
        assert_eq!(
            rewrite(r#"<link href="./public/style.css" rel="stylesheet" />"#),
            r#"<link href="public/style.css" rel="stylesheet" />"#
        );
    }

    #[test]
    fn a_base_url_prefixes_the_asset() {
        assert_eq!(
            rewrite_asset_urls(r#"<img src="../public/logo.png">"#, "/my-talk/"),
            r#"<img src="/my-talk/public/logo.png">"#
        );
    }

    /// A lint that cries wolf gets switched off, and so does a rewriter that
    /// mangles URLs it was never meant to touch.
    #[test]
    fn anything_that_is_not_a_deck_asset_is_left_alone() {
        for untouched in [
            "https://example.com/public/logo.png",
            "//cdn.example.com/public/logo.png",
            "#slide-3",
            "data:image/svg+xml;base64,AAAA",
            "mailto:someone@example.com",
            "/guide",
            "snippets/main.rs",
            // `public/` on its own names the directory, not a file in it.
            "public/",
        ] {
            let html = format!(r#"<a href="{untouched}">x</a>"#);
            assert_eq!(rewrite(&html), html, "{untouched}");
        }
    }

    /// A cache-buster or a fragment belongs to the URL, not to the path.
    #[test]
    fn a_query_or_fragment_survives_the_rewrite() {
        assert_eq!(
            rewrite(r#"<img src="../public/logo.png?v=2">"#),
            r#"<img src="public/logo.png?v=2">"#
        );
    }

    #[test]
    fn several_urls_in_one_document_are_all_rewritten() {
        assert_eq!(
            rewrite(r##"<img src="/public/a.png"><img src="../public/b.png"><a href="#x">y</a>"##),
            r##"<img src="public/a.png"><img src="public/b.png"><a href="#x">y</a>"##
        );
    }

    /// Whatever a half-written deck contains, exporting it must not lose text.
    #[test]
    fn an_unterminated_attribute_is_left_as_it_was() {
        let broken = r#"<img src="oops"#;
        assert_eq!(rewrite(broken), broken);
    }
}
