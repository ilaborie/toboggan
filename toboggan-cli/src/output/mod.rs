mod assets;
mod renderer;
pub use self::renderer::{OutputRenderer, RenderError};

mod text;
pub use self::text::TextRenderer;

pub(crate) mod html;
mod thumbnails;
pub use self::thumbnails::{ThumbnailOptions, generate_thumbnails};
mod typst;
use std::borrow::Cow;

use toboggan_core::{RenderTarget, Talk};

pub use self::typst::deck_root;
use crate::error::Result;
use crate::mermaid::MermaidRenderer;
use crate::settings::OutputFormat;

/// Serializes `talk` in `format`.
///
/// `base_url` is only meaningful for [`OutputFormat::Html`]: it is the URL the
/// exported file will be served from, used to resolve the deck's assets.
///
/// # Errors
/// Returns an error if the talk cannot be rendered in `format`.
pub fn serialize_talk(
    talk: &Talk,
    format: OutputFormat,
    base_url: &str,
    mermaid: &MermaidRenderer,
) -> Result<Vec<u8>> {
    match format {
        OutputFormat::Toml => TextRenderer::toml(talk),
        OutputFormat::Json => TextRenderer::json(talk),
        OutputFormat::Yaml => TextRenderer::yaml(talk),

        OutputFormat::Html => {
            let filtered = filter_for(talk, RenderTarget::Web);
            html::generate_html(&filtered, filtered.head.as_deref(), base_url)
        }

        OutputFormat::Typst => {
            let filtered = filter_for(talk, RenderTarget::Pdf);
            Ok(typst::generate_typst(&filtered, mermaid))
        }
    }
}

#[must_use]
pub fn get_extension(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Toml => "toml",
        OutputFormat::Json => "json",
        OutputFormat::Yaml => "yaml",
        OutputFormat::Html => "html",
        OutputFormat::Typst => "typ",
    }
}

/// Returns a `Talk` with slides hidden for `target` removed.
///
/// Thin wrapper over [`Talk::visible_in`], which the server applies to the deck
/// it serves — one definition of "hidden", so an exported deck and a presented
/// one cannot disagree about which slides exist.
fn filter_for(talk: &Talk, target: RenderTarget) -> Cow<'_, Talk> {
    talk.visible_in(target)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use anyhow::Context;
    use toboggan_core::{Content, Date, RenderTarget, Slide, SlideKind, Talk};

    use super::*;

    fn create_test_talk() -> anyhow::Result<Talk> {
        let mut talk = Talk::new("Test Talk");
        talk.date = Date::new(2024, 12, 25).with_context(|| "Failed to create test date")?;
        Ok(talk)
    }

    fn talk_with_hidden_slides() -> anyhow::Result<Talk> {
        use std::collections::BTreeSet;
        let mut talk = create_test_talk()?;
        // slide 1: visible everywhere
        talk.slides.push(Slide::new("Always Visible"));
        // slide 2: hidden in pdf (live slide)
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("Live Slide"),
            hidden_in: BTreeSet::from([RenderTarget::Pdf]),
            ..Default::default()
        });
        // slide 3: hidden in web (static equivalent)
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("Static Slide"),
            body_source: Some("# Static Slide\n\nCode here.".to_owned()),
            hidden_in: BTreeSet::from([RenderTarget::Web]),
            ..Default::default()
        });
        Ok(talk)
    }

    #[test]
    fn test_all_formats_serialize() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        let formats = [
            OutputFormat::Toml,
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Html,
            OutputFormat::Typst,
        ];

        for format in &formats {
            let result = serialize_talk(&talk, *format, "", &MermaidRenderer::default());
            assert!(result.is_ok(), "Failed to serialize format: {format:?}");
            assert!(
                !result.expect("ok").is_empty(),
                "Empty output for format: {format:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_extensions() {
        assert_eq!(get_extension(&OutputFormat::Toml), "toml");
        assert_eq!(get_extension(&OutputFormat::Json), "json");
        assert_eq!(get_extension(&OutputFormat::Yaml), "yaml");
        assert_eq!(get_extension(&OutputFormat::Html), "html");
        assert_eq!(get_extension(&OutputFormat::Typst), "typ");
    }

    #[test]
    fn test_html_excludes_web_hidden_slides() -> anyhow::Result<()> {
        let talk = talk_with_hidden_slides()?;
        let bytes = serialize_talk(&talk, OutputFormat::Html, "", &MermaidRenderer::default())?;
        let html = String::from_utf8(bytes).expect("utf8");

        assert!(
            html.contains("Always Visible"),
            "always-visible slide present in HTML"
        );
        assert!(html.contains("Live Slide"), "live slide present in HTML");
        assert!(
            !html.contains("Static Slide"),
            "static (web-hidden) slide absent from HTML"
        );
        Ok(())
    }

    #[test]
    fn test_typst_excludes_pdf_hidden_slides() -> anyhow::Result<()> {
        let talk = talk_with_hidden_slides()?;
        let bytes = serialize_talk(&talk, OutputFormat::Typst, "", &MermaidRenderer::default())?;
        let typ = String::from_utf8(bytes).expect("utf8");

        assert!(
            typ.contains("Always Visible"),
            "always-visible slide present in Typst"
        );
        assert!(
            !typ.contains("Live Slide"),
            "pdf-hidden live slide absent from Typst"
        );
        assert!(
            typ.contains("Static Slide"),
            "web-hidden static slide present in Typst"
        );
        Ok(())
    }

    #[test]
    fn test_toml_retains_all_slides() -> anyhow::Result<()> {
        let talk = talk_with_hidden_slides()?;
        let bytes = serialize_talk(&talk, OutputFormat::Toml, "", &MermaidRenderer::default())?;
        let toml = String::from_utf8(bytes).expect("utf8");

        assert!(toml.contains("Live Slide"), "live slide in TOML");
        assert!(toml.contains("Static Slide"), "static slide in TOML");
        Ok(())
    }

    #[test]
    fn test_filter_for_no_op_when_no_hidden_slides() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let result = filter_for(&talk, RenderTarget::Pdf);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "returns borrowed when nothing to filter"
        );
        Ok(())
    }
}
