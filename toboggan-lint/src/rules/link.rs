//! Checks that a slide's own assets exist on disk.

use std::path::{Path, PathBuf};

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// The one URL prefix the server maps to a directory on disk.
const PUBLIC_DIR: &str = "public";

/// An image or download the deck points at that is not in `public/`.
///
/// Only deck-relative URLs are checked. An absolute one (`https://…`), a
/// fragment, a `data:` payload or anything the server routes itself is somebody
/// else's business, and guessing at it would cost more in false positives than
/// the rule is worth.
pub(crate) struct BrokenLink;

impl Rule for BrokenLink {
    fn id(&self) -> RuleId {
        super::ids::LINK_BROKEN
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        // Without a `public/` directory there is nothing to resolve against, and
        // every relative URL would be reported. A deck that keeps its assets
        // elsewhere is not a deck with broken links.
        let Some(public) = context.public_dir() else {
            return;
        };

        let document = context.body_doc();
        // An `<img>` must be a file. An `<a href>` may be a server route the
        // deck does not own, so only ones aimed at `public/` are checked.
        let images = document
            .image_sources()
            .into_iter()
            .map(|url| ("image", url));
        let links = document
            .link_targets()
            .into_iter()
            .filter(|url| points_into_public(url))
            .map(|url| ("link", url));

        for (kind, url) in images.chain(links) {
            let Some((message, help)) = check_url(url, kind, public) else {
                continue;
            };
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    message,
                )
                .with_help(help),
            );
        }
    }
}

/// The complaint about `url`, or `None` when it is fine or not ours to judge.
fn check_url(url: &str, kind: &str, public: &Path) -> Option<(String, String)> {
    if !is_deck_relative(url) {
        return None;
    }
    let segments = url_segments(url);
    let (first, rest) = segments.split_first()?;

    if first == PUBLIC_DIR {
        if rest.is_empty() || join(public, rest).exists() {
            return None;
        }
        return Some((
            format!("{kind} \"{url}\" is not in the deck's public/ directory"),
            format!("expected {}", join(public, rest).display()),
        ));
    }

    // Under no other prefix can the deck serve a file: slides render at `/run`,
    // so `logo.png` is requested as `/logo.png`, and only `/public/` is mapped
    // to a directory on disk. Leaving the prefix off is the usual way to get
    // here, and then the file is sitting right there under a URL nobody asked
    // for — so say that, instead of "not found".
    let misplaced = join(public, &segments);
    let help = if misplaced.exists() {
        format!(
            "the file exists — reference it as \"../{PUBLIC_DIR}/{}\"",
            segments.join("/")
        )
    } else {
        format!(
            "deck assets are served from public/, so this would have to be \"../{PUBLIC_DIR}/{}\"",
            segments.join("/")
        )
    };
    Some((
        format!("{kind} \"{url}\" is not served: only public/ is"),
        help,
    ))
}

/// Joins URL segments onto a filesystem path.
fn join(base: &Path, segments: &[String]) -> PathBuf {
    segments
        .iter()
        .fold(base.to_path_buf(), |path, segment| path.join(segment))
}

/// Whether a URL is a deck-relative reference at all, as opposed to an absolute
/// address, a fragment, or an inline payload.
fn is_deck_relative(url: &str) -> bool {
    let url = url.trim();
    !url.is_empty()
        && !url.starts_with('#')
        && !url.starts_with("//")
        && !url.contains("://")
        && !url.starts_with("data:")
        && !url.starts_with("mailto:")
        && !url.starts_with("tel:")
        // A percent escape would have to be decoded before it could be matched
        // against a filename, and getting that wrong means inventing a broken
        // link that is not broken.
        && !url.contains('%')
}

/// Whether a link aims at the deck's own assets, once resolved.
fn points_into_public(url: &str) -> bool {
    is_deck_relative(url) && url_segments(url).first().is_some_and(|it| it == PUBLIC_DIR)
}

/// Splits a URL path into normalized segments, dropping the query and fragment
/// and resolving `.` and `..` the way a browser would.
///
/// Slides render at `/run`, so the browser resolves every relative URL against
/// `/` — which is why `../public/logo.png`, `public/logo.png` and
/// `/public/logo.png` all name the same file.
fn url_segments(url: &str) -> Vec<String> {
    let path = url
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_start_matches('/');

    let mut segments: Vec<String> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            // Clamped at the root, as a browser does — `../../x` from `/run`
            // is still `/x`.
            ".." => {
                segments.pop();
            }
            other => segments.push(other.to_owned()),
        }
    }
    segments
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::fs;

    use tempfile::TempDir;
    use toboggan_core::{Content, Slide, Talk};

    use super::*;
    use crate::diagnostic::SlideRef;
    use crate::rule::{LintConfig, RuleContext};

    /// A deck with `public/logo.png` on disk, and one slide holding `body`.
    fn diagnostics_for(body: &str) -> (TempDir, Vec<LintDiagnostic>) {
        let deck = TempDir::new().expect("temp dir");
        let slides = deck.path().join("slides");
        fs::create_dir(&slides).expect("slides dir");
        fs::create_dir(deck.path().join("public")).expect("public dir");
        fs::write(deck.path().join("public/logo.png"), "x").expect("logo");

        let slide = Slide::new("T").with_body(Content::html(body));
        let mut talk = Talk::new("Test").with_source_dir(slides.to_string_lossy().into_owned());
        talk.slides = vec![slide.clone()];

        let slide_ref = SlideRef::new(0, &slide);
        let config = LintConfig::default();
        let context = RuleContext::new(&talk, &slide, &slide_ref, &config);
        let mut out = Vec::new();
        BrokenLink.check_slide(&context, &mut out);
        (deck, out)
    }

    /// The three spellings a browser treats identically, because slides are
    /// served from `/run` and so resolve against `/`.
    #[test]
    fn every_spelling_of_an_existing_asset_resolves() {
        for url in [
            "../public/logo.png",
            "public/logo.png",
            "/public/logo.png",
            "../../public/logo.png",
        ] {
            let (_deck, diagnostics) = diagnostics_for(&format!(r#"<img src="{url}">"#));
            assert!(
                diagnostics.is_empty(),
                "{url} should resolve: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn a_missing_asset_is_reported() {
        let (_deck, diagnostics) = diagnostics_for(r#"<img src="../public/nope.png">"#);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics.iter().all(|it| it.message.contains("nope.png")));
    }

    /// The most common mistake: the `public/` prefix left off entirely. The
    /// file exists, but at a URL the server does not serve.
    #[test]
    fn a_reference_without_the_public_prefix_is_reported() {
        let (_deck, diagnostics) = diagnostics_for(r#"<img src="logo.png">"#);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    }

    #[test]
    fn remote_and_inline_images_are_left_alone() {
        for url in [
            "https://example.com/logo.png",
            "//cdn.example.com/logo.png",
            "data:image/svg+xml;base64,AAAA",
        ] {
            let (_deck, diagnostics) = diagnostics_for(&format!(r#"<img src="{url}">"#));
            assert!(diagnostics.is_empty(), "{url} is not ours: {diagnostics:?}");
        }
    }

    /// A query or fragment on an asset URL is not part of the filename.
    #[test]
    fn a_query_string_does_not_break_resolution() {
        let (_deck, diagnostics) = diagnostics_for(r#"<img src="../public/logo.png?v=2">"#);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    /// A link may point at a server route the deck does not own — `/guide`,
    /// `/download.pdf`, an anchor — so only ones aimed at `public/` are checked.
    #[test]
    fn only_links_into_public_are_checked() {
        let (_deck, diagnostics) = diagnostics_for(
            r##"<a href="/guide">guide</a><a href="#top">top</a><a href="slides">deck</a>"##,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let (_deck, diagnostics) = diagnostics_for(r#"<a href="../public/handout.pdf">get it</a>"#);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    }

    /// A deck that keeps its assets somewhere else is not a deck with broken
    /// links, so with no `public/` to resolve against the rule says nothing.
    #[test]
    fn without_a_public_directory_nothing_is_reported() {
        let deck = TempDir::new().expect("temp dir");
        let slides = deck.path().join("slides");
        fs::create_dir(&slides).expect("slides dir");

        let slide = Slide::new("T").with_body(Content::html(r#"<img src="whatever.png">"#));
        let mut talk = Talk::new("Test").with_source_dir(slides.to_string_lossy().into_owned());
        talk.slides = vec![slide.clone()];

        let slide_ref = SlideRef::new(0, &slide);
        let config = LintConfig::default();
        let context = RuleContext::new(&talk, &slide, &slide_ref, &config);
        let mut out = Vec::new();
        BrokenLink.check_slide(&context, &mut out);
        assert!(out.is_empty(), "{out:?}");
    }
}
