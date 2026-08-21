use comrak::{Arena, Options, parse_document};
use toboggan_cli::Result;
use toboggan_cli::mermaid::MermaidRenderer;
use toboggan_cli::parser::{SlideContentParser, SlideContext};
use toboggan_core::Content;

#[allow(clippy::print_stdout, clippy::result_large_err)]
fn main() -> Result<()> {
    let raw = include_str!("./slide.md");

    let arena = Arena::new();
    let options = Options::default();

    let doc = parse_document(&arena, raw, &options);

    let content_parser = SlideContentParser::new();
    let (slide, _) = content_parser.parse_with_defaults(
        doc.children(),
        SlideContext {
            name: Some("example-slide"),
            path: None,
            asset_root: None,
            mermaid: &MermaidRenderer::default(),
        },
    )?;

    println!("{slide:#?}");
    println!("===");

    if let Content::Html { raw, .. } = slide.body {
        println!("{raw}");
    }

    Ok(())
}
