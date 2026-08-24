use std::path::{Path, PathBuf};
/// The Typst project root for `talk`: the deck root its slides' relative asset
/// paths resolve against.
///
/// `source_dir` is the *slides* folder, but a slide writes `../public/logo.png`,
/// so the root has to be its parent. Compiling with any other root makes typst
/// reject every such image with "path would escape the project root".
///
/// `None` when the talk was not built from a folder (a deserialized `.toml`
/// does not carry `source_dir`); the caller then compiles without `--root`,
/// which is correct for a deck that references no local files.
#[must_use]
pub fn deck_root(talk: &Talk) -> Option<PathBuf> {
    let slides = Path::new(talk.source_dir.as_deref()?);
    // Mirrors `parser::directory::deck_root`: a relative folder with no
    // directory component has an empty parent rather than none, and that still
    // means the current directory. Treating it as "no parent" rooted the
    // project inside the slides folder, so `-p slides` and `-p ./slides/`
    // compiled against different roots.
    Some(match slides.parent() {
        None => slides.to_path_buf(),
        Some(parent) if parent.as_os_str().is_empty() => PathBuf::from("."),
        Some(parent) => parent.to_path_buf(),
    })
}

use std::cell::RefCell;
use std::fmt::Write as _;

use comrak::nodes::{AlertType, AstNode, ListType, NodeValue};
use comrak::{Arena, parse_document};
use serde::{Deserialize, Serialize};
use toboggan_core::{Content, Slide, SlideBody, SlideKind, Talk};

use crate::error::{Result, TobogganCliError};
use crate::mermaid::{MermaidFence, MermaidRenderer};
use crate::parser::default_options;

/// What [`render_node`] needs beyond the node it is rendering.
///
/// `list_depth` and `tight` used to be positional arguments on every helper;
/// bundling them with the deck's Mermaid settings keeps one value to thread
/// instead of three.
#[derive(Debug, Clone, Copy)]
struct RenderCtx<'a> {
    /// Deck-level Mermaid settings for ` ```mermaid ` fences.
    mermaid: &'a MermaidRenderer,
    /// The slide the markdown came from, used in diagnostics.
    source_name: &'a str,
    /// Where a writer reports something it could not render.
    ///
    /// Every writer below returns `()`, and threading `Result` through all of
    /// them — including two mutually recursive ones over a comrak arena — to
    /// carry a failure that only [`write_mermaid`] can raise would touch thirty
    /// signatures. A shared reference is itself `Copy`, so collecting here
    /// keeps `RenderCtx` `Copy` and leaves every writer alone.
    errors: &'a RefCell<Vec<TobogganCliError>>,
    /// Nesting depth of the enclosing list.
    list_depth: usize,
    /// Whether the enclosing list is tight, i.e. has no blank line between items.
    tight: bool,
}

impl<'a> RenderCtx<'a> {
    const fn new(
        mermaid: &'a MermaidRenderer,
        source_name: &'a str,
        errors: &'a RefCell<Vec<TobogganCliError>>,
    ) -> Self {
        Self {
            mermaid,
            source_name,
            errors,
            list_depth: 0,
            tight: false,
        }
    }

    const fn with_tight(self, tight: bool) -> Self {
        Self { tight, ..self }
    }

    /// One level further into a list, where items are always tight.
    const fn nested(self) -> Self {
        Self {
            list_depth: self.list_depth + 1,
            tight: true,
            ..self
        }
    }

    /// Inside a table cell, which starts its own list nesting.
    const fn in_cell(self) -> Self {
        Self {
            list_depth: 0,
            tight: true,
            ..self
        }
    }
}

/// The slide file a Typst diagnostic should name.
fn slide_name(slide: &Slide) -> String {
    slide.source_path.as_deref().map_or_else(
        || slide.title.display_text().to_owned(),
        |path| path.to_string_lossy().into_owned(),
    )
}

/// The label `toboggan pdf` queries to find where each slide starts and ends.
///
/// Public because the query that reads these markers lives in another crate.
/// The label, the field names and the `start`/`end` values are one wire format
/// with a producer here and a consumer there; sharing the pieces is what makes a
/// rename fail to compile instead of failing to find anything.
pub const SLIDE_MARKER_LABEL: &str = "toboggan-slide";

/// An invisible marker saying which slide a page belongs to.
///
/// `#metadata` renders nothing and occupies no space — a deck compiles to the
/// very same pages with and without these — but it *is* located, so
/// `typst eval --in deck.typ 'query(<toboggan-slide>)…'` can report the page
/// each slide starts and ends on. That is what turns "23 slides silently became
/// 38 pages" into a warning naming the slide that overflowed.
fn slide_marker(name: &str, at: MarkerAt) -> String {
    let at = at.as_str();
    let name = escape_typst_string(name);
    format!("  #metadata((slide: \"{name}\", at: \"{at}\"))<{SLIDE_MARKER_LABEL}>")
}

/// Which end of a slide a marker labelled [`SLIDE_MARKER_LABEL`] marks.
///
/// `Deserialize` so the reader of these markers gets the closed set back rather
/// than comparing strings: an `at` typst never emits fails to parse here instead
/// of falling through to "these markers do not pair up" three fields away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerAt {
    Start,
    End,
}

impl MarkerAt {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Renders the whole deck as a Typst document.
///
/// # Errors
/// Returns [`TobogganCliError::SlidesFailedToParse`] if any diagram could not
/// be drawn. Emitting a red placeholder and succeeding instead put a visibly
/// broken picture in the handout — the artifact an audience physically holds —
/// while the command still exited 0 and the only clue was a `warn!` the default
/// log filter drops.
pub(super) fn generate_typst(talk: &Talk, mermaid: &MermaidRenderer) -> Result<Vec<u8>> {
    let errors = RefCell::new(Vec::new());
    let mut out = String::new();
    write_header(&mut out, talk.typst_preamble.as_deref());
    // The cover is not written by the loop below — its title and date are the
    // title slide's — so its body has to be rendered here or it is lost.
    let cover = talk
        .slides
        .iter()
        .find(|slide| matches!(slide.kind, SlideKind::Cover));
    let cover_body = cover.map_or_else(String::new, |cover| slide_body(cover, mermaid, &errors));
    let cover_name = cover.map_or_else(|| talk.title.clone(), slide_name);
    write_title_slide(
        &mut out,
        &talk.title,
        &talk.date.to_string(),
        &cover_body,
        &cover_name,
    );
    for slide in &talk.slides {
        write_slide(&mut out, slide, mermaid, &errors);
    }
    let failures = errors.into_inner();
    if !failures.is_empty() {
        return Err(TobogganCliError::SlidesFailedToParse { failures });
    }
    Ok(out.into_bytes())
}

/// Renders a SINGLE slide as a self-contained, fixed-size (16:9) Typst document
/// for the thumbnail pipeline.
///
/// Deliberately avoids the touying theme (which is unreliable for a single
/// isolated slide) and instead uses a plain fixed-size page so the slide always
/// renders on page 1; overflowing content is simply clipped in the thumbnail.
/// The markdown-to-Typst conversion (code blocks, alerts, lists, …) is reused.
/// # Errors
/// Returns [`TobogganCliError::SlidesFailedToParse`] if the slide's diagram
/// could not be drawn, so an overview page does not quietly show a red box.
pub(super) fn generate_thumbnail_typst(
    slide: &Slide,
    mermaid: &MermaidRenderer,
) -> Result<Vec<u8>> {
    let errors = RefCell::new(Vec::new());
    let mut out = String::new();
    out.push_str(
        r#"#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *
#import "@preview/gentle-clues:1.3.1": *
#import "@preview/mitex:0.2.7": mi, mitex

#show: codly-init.with()
#codly(languages: codly-languages)

#set page(width: 32cm, height: 18cm, margin: 1.4cm, fill: white)
#set text(size: 20pt)

"#,
    );

    let title = content_to_typst(&slide.title);
    if !title.trim().is_empty() {
        let _ = writeln!(out, "#text(weight: \"bold\", size: 1.5em)[{title}]\n");
    }

    out.push_str(&slide_body(slide, mermaid, &errors));

    let failures = errors.into_inner();
    if !failures.is_empty() {
        return Err(TobogganCliError::SlidesFailedToParse { failures });
    }
    Ok(out.into_bytes())
}

/// Writes the document preamble: the imports, the touying theme and the codly
/// setup.
///
/// A deck that brings its own — `_preamble.typ`, or `--typst-preamble` — gets it
/// emitted verbatim instead. Replacement rather than addition is what makes the
/// hatch worth having: the theme and the aspect ratio are chosen here, and
/// nothing written afterwards can take them back.
///
/// A replacement therefore owns every *line* of [`DEFAULT_PREAMBLE`], not only
/// its imports: drop `#show: codly-init.with()` or `#codly(languages:
/// codly-languages)` and code fences lose their line numbers and language
/// labels, and drop `subslide-preamble: none` and every slide title prints
/// twice.
///
/// A replacement that picks *another theme* still owes the body that second
/// suppression, but the argument is the theme's own: `themes.metropolis`
/// ignores `subslide-preamble` and needs `header: none`. What a preamble owes
/// here is the outcome, not the keyword.
///
/// Why the keyword exists at all, since it looks like a theme detail and is
/// not. The theme's default preamble is
/// `text(1.2em, weight: "bold", utils.display-current-heading(level: 2))`, and
/// the level-2 heading it displays is the very `== <title>` that
/// [`write_standard_body`] emits inside `#slide[..]` — so the theme printed the
/// title above the body and the body printed it again. It has to be passed
/// *there*: `simple-theme` takes it as a named argument and stores it, and its
/// own `slide` re-applies the stored value to every slide, overwriting any later
/// `config-common`.
fn write_header(out: &mut String, preamble: Option<&str>) {
    let Some(preamble) = preamble else {
        let _ = writeln!(out, "{DEFAULT_PREAMBLE}");
        return;
    };
    // Verbatim, with exactly one blank line after it however the file ended:
    // this is an escape hatch, and rewriting what it says would defeat it.
    let _ = writeln!(out, "{}\n", preamble.trim_end());
}

/// The preamble a deck gets when it does not bring its own.
const DEFAULT_PREAMBLE: &str = r#"// Generated by toboggan-cli — compile with: typst compile presentation.typ
//
// First-time compilation will download the touying, codly, codly-languages,
// gentle-clues and mitex packages from Typst Universe (requires network access
// or a populated cache).

#import "@preview/touying:0.7.3": *
#import themes.simple: *
#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *
#import "@preview/gentle-clues:1.3.1": *
#import "@preview/mitex:0.2.7": mi, mitex

// The slide title is emitted by the slide body itself; `subslide-preamble: none`
// stops the theme from displaying it a second time.
#show: simple-theme.with(aspect-ratio: "16-9", subslide-preamble: none)
#show: codly-init.with()
#codly(languages: codly-languages)
"#;

/// Writes the cover page: the deck's title and date, then whatever else
/// `_cover.md` holds.
///
/// `cover_body` used to be dropped on the floor — the arm for a Cover slide did
/// nothing, on the grounds that the title and date were already emitted here.
/// True of the title and date, and of nothing else: a cover whose point is a
/// full-bleed illustration exported as a blank page with a date on it.
fn write_title_slide(
    out: &mut String,
    title: &str,
    date: &str,
    cover_body: &str,
    cover_name: &str,
) {
    let title = escape_typst(title);
    let body_block = if cover_body.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n  {}", cover_body.trim_end())
    };
    let start = slide_marker(cover_name, MarkerAt::Start);
    let end = slide_marker(cover_name, MarkerAt::End);
    let _ = writeln!(
        out,
        r#"#title-slide[
{start}
  #text(size: 2em, weight: "bold")[{title}]
  #v(1em)
  #text(size: 0.8em, fill: gray)[{date}]{body_block}
{end}
]"#
    );
}

fn write_slide(
    out: &mut String,
    slide: &Slide,
    mermaid: &MermaidRenderer,
    errors: &RefCell<Vec<TobogganCliError>>,
) {
    match slide.kind {
        // The whole cover — title, date and body — is emitted by
        // `write_title_slide`, so there is nothing left to render here.
        SlideKind::Cover => {}
        SlideKind::Part => write_section(out, slide, mermaid, errors),
        SlideKind::Standard => write_standard(out, slide, mermaid, errors),
    }
}

fn write_section(
    out: &mut String,
    slide: &Slide,
    mermaid: &MermaidRenderer,
    errors: &RefCell<Vec<TobogganCliError>>,
) {
    let title = content_to_typst(&slide.title);
    let body = slide_body(slide, mermaid, errors);
    let body_block = if body.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n  {body}")
    };
    let start = slide_marker(&slide_name(slide), MarkerAt::Start);
    let end = slide_marker(&slide_name(slide), MarkerAt::End);
    let _ = writeln!(
        out,
        "#new-section-slide[
{start}
  #text(size: 1.5em, weight: \"bold\")[{title}]{body_block}
{end}
]"
    );
}

/// Drop the leading `= heading` produced by `md_to_typst` for `_part.md` files
/// (whose markdown source starts with the part title).
///
/// Without this, the section block would render the title twice — once via the
/// outer `#text(...)[{title}]` and once via the `= H1` from `body_source`.
fn strip_leading_heading(typst: &str) -> String {
    let trimmed = typst.trim_start();
    let Some(rest) = trimmed.strip_prefix("= ") else {
        return typst.to_owned();
    };
    let Some(nl) = rest.find('\n') else {
        return String::new();
    };
    rest[nl + 1..].trim_start().to_owned()
}

fn write_standard(
    out: &mut String,
    slide: &Slide,
    mermaid: &MermaidRenderer,
    errors: &RefCell<Vec<TobogganCliError>>,
) {
    // Slides with live terminals that weren't filtered out get a placeholder.
    if slide.terminals.is_empty() {
        write_standard_body(out, slide, mermaid, errors);
    } else {
        write_terminal_placeholder(out, slide);
    }
}

fn write_terminal_placeholder(out: &mut String, slide: &Slide) {
    let title = content_to_typst(&slide.title);
    tracing::warn!(
        "slide '{title}' has live terminals but is not marked \
         `hidden_in = [\"pdf\"]` — emitting a placeholder page in the PDF; \
         add `hidden_in = [\"pdf\"]` to suppress this"
    );
    let start = slide_marker(&slide_name(slide), MarkerAt::Start);
    let end = slide_marker(&slide_name(slide), MarkerAt::End);
    let _ = writeln!(
        out,
        "#slide[
{start}
  == {title}

  #align(center + horizon)[
    #text(size: 1.2em, fill: gray)[\\[Live terminal demo --- see recording / live presentation\\]]
  ]
{end}
]"
    );
}

fn write_standard_body(
    out: &mut String,
    slide: &Slide,
    mermaid: &MermaidRenderer,
    errors: &RefCell<Vec<TobogganCliError>>,
) {
    let title = content_to_typst(&slide.title);
    let body = slide_body(slide, mermaid, errors);
    let title_block = if title.trim().is_empty() {
        String::new()
    } else {
        format!("  == {title}\n\n")
    };
    let body_block = if body.trim().is_empty() {
        tracing::error!(
            "Slide '{title}' rendered to an empty body; emitting a placeholder \
             page so the deck does not silently lose a slide."
        );
        format!(
            "  #align(center + horizon)[\n    \
             #text(size: 1.2em, fill: red)[\\[Empty slide '{title}' --- check source\\]]\n  ]\n"
        )
    } else {
        body
    };
    let start = slide_marker(&slide_name(slide), MarkerAt::Start);
    let end = slide_marker(&slide_name(slide), MarkerAt::End);
    let _ = writeln!(out, "#slide[\n{start}\n{title_block}{body_block}\n{end}\n]");
}

/// Render a slide's body, whatever kind of slide it is.
///
/// Routes on the `SlideBody` view so the three meaningful states are explicit.
/// The leading H1 from a markdown source is stripped in every case because each
/// caller emits the title itself — `== <title>` inside `#slide[..]`, the outer
/// `#text(..)` of a section slide, or the title slide's own heading.
fn slide_body(
    slide: &Slide,
    mermaid: &MermaidRenderer,
    errors: &RefCell<Vec<TobogganCliError>>,
) -> String {
    match slide.body_view() {
        SlideBody::Empty => String::new(),
        SlideBody::Rendered(content) => content_to_typst(content),
        SlideBody::FromMarkdown { source, .. } => strip_leading_heading(&md_to_typst(
            source,
            RenderCtx::new(mermaid, &slide_name(slide), errors),
        )),
    }
}

/// Convert `Content` to escaped Typst markup.
///
/// Uses `alt` text when available; falls back to a visible red placeholder
/// when only raw HTML is present, so the gap is obvious in the rendered PDF
/// even if the warning is filtered out of the logs.
fn content_to_typst(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => escape_typst(text),
        Content::Html { alt: Some(alt), .. } => escape_typst(alt),
        Content::Html { .. } => {
            tracing::warn!(
                "Slide content is HTML without alt text; emitting a visible Typst \
                 placeholder. Add alt text or use body_source for proper rendering."
            );
            "#text(fill: red)[\\[HTML content not exportable --- add alt text\\]]".to_owned()
        }
    }
}

/// Emit a Mermaid diagram, drawn while the deck builds and embedded as SVG.
///
/// Without an explicit `width=`, the diagram keeps its natural size but is
/// clamped to the width available to it — what `max-width: 100%` does on the
/// web. `#layout` is what makes `available.width` knowable at that point.
///
/// The diagram was already drawn once, for the HTML the parser produced, so a
/// failure here means the two pipelines disagree rather than that the author
/// wrote a bad diagram. It gets the same visible red placeholder as any other
/// content this renderer cannot express, so the gap is obvious in the PDF.
fn write_mermaid(out: &mut String, ctx: RenderCtx<'_>, fence: Result<MermaidFence>, diagram: &str) {
    let drawn = fence.and_then(|fence| {
        let svg = ctx.mermaid.render_svg(&fence, diagram, ctx.source_name)?;
        Ok((fence, svg))
    });
    let (fence, svg) = match drawn {
        Ok(drawn) => drawn,
        Err(error) => {
            // Recorded rather than logged: `generate_typst` turns a non-empty
            // accumulator into a failure, so `toboggan pdf` refuses and
            // `/download.pdf` answers 503 instead of serving a handout with a
            // red box where the diagram should be.
            ctx.errors.borrow_mut().push(error);
            out.push_str("#text(fill: red)[\\[Mermaid diagram could not be drawn\\]]\n\n");
            return;
        }
    };
    let data = escape_typst_string(&svg);
    // Typst carries `alt` into the PDF's accessibility tree, so the label the
    // author wrote for the web is not web-only.
    let alt = fence.alt().map_or_else(String::new, |alt| {
        format!(", alt: \"{}\"", escape_typst_string(alt))
    });
    match fence.width() {
        Some(width) => {
            let _ = writeln!(
                out,
                "#align(center, image(bytes(\"{data}\"), format: \"svg\", width: {width}{alt}))\n"
            );
        }
        None => {
            let _ = writeln!(
                out,
                "#align(center, layout(available => {{\n\
                 let diagram = bytes(\"{data}\")\n\
                 let natural = measure(image(diagram, format: \"svg\")).width\n\
                 image(diagram, format: \"svg\", width: calc.min(natural, available.width){alt})\n\
                 }}))\n"
            );
        }
    }
}

/// Escape text for use inside a Typst double-quoted string argument.
///
/// Typst string literals use `\` as the escape character, so both `"` and `\`
/// must be escaped. Line breaks are escaped too, because a code block passed to
/// `#raw(block: true, …)` arrives here with its own. This is separate from
/// `escape_typst`, which escapes Typst markup characters for content nodes.
fn escape_typst_string(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('"', r#"\""#)
        .replace('\r', r"\r")
        .replace('\n', r"\n")
}

/// Convert a `CommonMark` source string to Typst markup.
fn md_to_typst(source: &str, ctx: RenderCtx<'_>) -> String {
    let arena = Arena::new();
    let options = default_options();
    let root = parse_document(&arena, source, &options);
    let mut out = String::new();
    render_node(root, &mut out, ctx);
    out
}

type MarkdownNode<'a> = AstNode<'a>;

fn render_children<'a>(node: &'a MarkdownNode<'a>, out: &mut String, ctx: RenderCtx<'_>) {
    for child in node.children() {
        render_node(child, out, ctx);
    }
}

#[allow(clippy::too_many_lines)]
fn render_node<'a>(node: &'a MarkdownNode<'a>, out: &mut String, ctx: RenderCtx<'_>) {
    match &node.data.borrow().value {
        NodeValue::FrontMatter(_) | NodeValue::HtmlBlock(_) | NodeValue::HtmlInline(_) => {
            // Typst has no HTML, so raw markup is dropped. Note this is *not*
            // where the directive comments go: `<!-- term: … -->`, `<!-- pause -->`
            // and friends are consumed by the parser and never reach the
            // renderer. What lands here is genuine slide HTML (`<style>`, `<div>`,
            // an inline `<img>`), which silently does not appear in the PDF.
        }

        NodeValue::Paragraph => {
            render_children(node, out, ctx);
            if ctx.tight {
                out.push('\n');
            } else {
                out.push_str("\n\n");
            }
        }

        NodeValue::Heading(heading) => {
            for _ in 0..heading.level {
                out.push('=');
            }
            out.push(' ');
            render_children(node, out, ctx);
            out.push('\n');
        }

        NodeValue::Text(text) => out.push_str(&escape_typst(text)),

        NodeValue::SoftBreak => out.push(' '),

        NodeValue::LineBreak => out.push_str("\\\n"),

        NodeValue::Strong => {
            out.push_str("#strong[");
            render_children(node, out, ctx);
            out.push(']');
        }

        NodeValue::Emph => {
            out.push_str("#emph[");
            render_children(node, out, ctx);
            out.push(']');
        }

        NodeValue::Strikethrough => {
            out.push_str("#strike[");
            render_children(node, out, ctx);
            out.push(']');
        }

        NodeValue::Code(code) => write_inline_code(out, &code.literal),

        NodeValue::CodeBlock(cb) => match ctx.mermaid.parse_info(&cb.info, ctx.source_name) {
            Some(fence) => write_mermaid(out, ctx, fence, &cb.literal),
            None => write_fenced_code(out, cb.info.trim(), &cb.literal),
        },

        NodeValue::BlockQuote => {
            out.push_str("#quote(block: true)[\n");
            render_children(node, out, ctx);
            out.push_str("]\n\n");
        }

        NodeValue::ThematicBreak => out.push_str("#line(length: 100%)\n\n"),

        NodeValue::List(list) => {
            let is_ordered = list.list_type == ListType::Ordered;
            render_list(node, out, ctx, is_ordered);
        }

        NodeValue::Link(link) => {
            let url = escape_typst_string(&link.url);
            let _ = write!(out, "#link(\"{url}\")[");
            render_children(node, out, ctx);
            out.push(']');
        }

        NodeValue::Image(link) => {
            let url = escape_typst_string(&link.url);
            let _ = writeln!(out, "#image(\"{url}\")");
        }

        NodeValue::Table(_) => {
            render_table(node, out, ctx);
        }

        NodeValue::Alert(alert) => {
            render_alert(node, out, ctx, alert.alert_type, alert.title.as_deref());
        }

        // Handed to MiTeX rather than dropped between Typst's own `$…$`.
        // Typst math is not LaTeX — `\frac{a}{b}` there is a call to an unknown
        // variable `rac` — so passing the source through made `toboggan pdf`
        // fail to compile for any deck that used more than bare symbols.
        NodeValue::Math(math) => {
            write_math(out, &math.literal, math.display_math);
        }

        _ => render_children(node, out, ctx),
    }
}

/// Render a GFM alert block as a gentle-clues callout.
///
/// The five GFM alert kinds map to the closest gentle-clues predefined clue
/// (gentle-clues does not expose `note`/`important`/`caution` by name), with
/// the title overridden to match the GFM label.
fn render_alert<'a>(
    node: &'a MarkdownNode<'a>,
    out: &mut String,
    ctx: RenderCtx<'_>,
    kind: AlertType,
    title_override: Option<&str>,
) {
    let label = title_override.unwrap_or_else(|| kind.default_title());
    let clue = clue_fn(kind);
    // The title is the author's, and it lands inside a Typst string literal:
    // `> [!NOTE] He said "hi"` closes the argument early and fails the whole
    // compile. Same class as the backtick that used to cost the guide its PDF.
    let label = escape_typst_string(label);
    let _ = writeln!(out, "#{clue}(title: \"{label}\")[");
    render_children(node, out, ctx.with_tight(false));
    out.push_str("]\n\n");
}

/// Map a GFM alert kind to a gentle-clues predefined clue function.
const fn clue_fn(kind: AlertType) -> &'static str {
    match kind {
        AlertType::Note => "info",
        AlertType::Tip => "tip",
        AlertType::Important => "notify",
        AlertType::Warning => "warning",
        AlertType::Caution => "danger",
    }
}

fn render_list<'a>(
    node: &'a MarkdownNode<'a>,
    out: &mut String,
    ctx: RenderCtx<'_>,
    ordered: bool,
) {
    for child in node.children() {
        let indent = "  ".repeat(ctx.list_depth);
        out.push_str(&indent);
        if ordered {
            out.push_str("+ ");
        } else {
            out.push_str("- ");
        }
        render_children(child, out, ctx.nested());
    }
    out.push('\n');
}

fn render_table<'a>(node: &'a MarkdownNode<'a>, out: &mut String, ctx: RenderCtx<'_>) {
    let mut rows: Vec<Vec<String>> = Vec::new();

    for row_node in node.children() {
        let mut cells: Vec<String> = Vec::new();
        for cell_node in row_node.children() {
            let mut cell = String::new();
            render_children(cell_node, &mut cell, ctx.in_cell());
            cells.push(cell.trim().to_owned());
        }
        rows.push(cells);
    }

    let Some(first) = rows.first() else { return };
    let columns = first.len();
    if columns == 0 {
        // A visible placeholder, like the terminal and unsupported-content paths:
        // silently dropping the table leaves a hole in the PDF that nobody
        // notices until they are presenting from it.
        tracing::warn!(
            "Table has no columns (empty header row); emitting a placeholder instead of an \
             invalid #table."
        );
        let _ = writeln!(
            out,
            "#text(fill: red)[\\[Table omitted --- header row has no columns\\]]"
        );
        return;
    }

    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: {columns},");
    for row in &rows {
        for cell in row.iter().take(columns) {
            let _ = writeln!(out, "  [{cell}],");
        }
        for _ in row.len()..columns {
            let _ = writeln!(out, "  [],");
        }
    }
    let _ = writeln!(out, ")\n");
}

/// Write inline code to `out` as Typst.
///
/// A backtick in the content goes through `#raw("…")` rather than a longer
/// delimiter. Typst has no equivalent of `CommonMark`'s rule that more
/// backticks open a longer span: to Typst, ``` `` ``` is an *empty* raw span,
/// so the fence closes immediately and the literal backtick that follows opens
/// a new one that runs to the end of the document. Emitting that produced a
/// file that did not compile, and the error surfaced hundreds of lines later
/// wherever the runaway span happened to hit a `$`.
fn write_inline_code(out: &mut String, code: &str) {
    if code.contains('`') {
        let _ = write!(out, r#"#raw("{}")"#, escape_typst_string(code));
        return;
    }
    out.push('`');
    out.push_str(code);
    out.push('`');
}

/// Write a `$…$` / `$$…$$` expression to `out` as a `MiTeX` call.
///
/// `mi` renders inline, `mitex` renders as a centred block. `MiTeX` takes the
/// LaTeX as a Typst raw span, whose delimiter is a backtick — so a literal
/// containing one cannot be passed through. TeX writes an opening quote as a
/// backtick, which makes that rare but not impossible; rather than emit Typst
/// that fails to compile, such an expression falls back to showing its own
/// source, which is visible in the PDF and obviously wrong to the author.
fn write_math(out: &mut String, latex: &str, display_math: bool) {
    if latex.contains('`') {
        let _ = write!(out, r#"#raw("{}")"#, escape_typst_string(latex));
        if display_math {
            out.push_str("\n\n");
        }
        return;
    }

    let function = if display_math { "mitex" } else { "mi" };
    let _ = write!(out, "#{function}(`{latex}`)");
    if display_math {
        out.push_str("\n\n");
    }
}

/// Write a fenced code block to `out` as Typst.
///
/// Code containing a backtick goes through `#raw(block: true, …)` for the same
/// reason [`write_inline_code`] does: Typst reads one or three backticks and
/// nothing else, so it has no equivalent of `CommonMark`'s "a longer fence
/// wins" rule. Padding the fence out past three — which this used to do —
/// closes the block at the third backtick and leaves the rest as markup. A deck
/// that documents markdown contains a fence inside a fence, so the guide itself
/// is the trigger.
fn write_fenced_code(out: &mut String, lang: &str, code: &str) {
    if code.contains('`') {
        let lang = if lang.is_empty() {
            String::new()
        } else {
            format!(r#"lang: "{}", "#, escape_typst_string(lang))
        };
        let _ = writeln!(
            out,
            r#"#raw(block: true, {lang}"{}")"#,
            escape_typst_string(code)
        );
        out.push('\n');
        return;
    }

    let _ = write!(out, "```{lang}\n{code}");
    if !code.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n\n");
}

/// Escape Typst markup special characters with a backslash.
fn escape_typst(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '*' | '_' | '`' | '#' | '@' | '<' | '>' | '[' | ']' | '{' | '}' | '$' | '~' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;

    use toboggan_core::{Date, RenderTarget, Slide, SlideKind, Style, Talk, TerminalConfig};

    use super::*;

    /// The renderer with stock Mermaid settings, which is what every test here
    /// wants; shadows the two-argument version so the tests stay readable.
    fn md_to_typst(source: &str) -> String {
        super::md_to_typst(
            source,
            RenderCtx::new(
                &MermaidRenderer::default(),
                "test.md",
                &RefCell::new(Vec::new()),
            ),
        )
    }

    fn make_talk(title: &str) -> Talk {
        let mut talk = Talk::new(title);
        talk.date = Date::new(2024, 1, 15).expect("valid date");
        talk
    }

    fn slide_with_source(source: &str) -> Slide {
        Slide {
            kind: SlideKind::Standard,
            body_source: Some(source.to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn test_cover_slide_preamble() {
        let talk = make_talk("My Presentation");
        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(output.contains("My Presentation"), "title in preamble");
        assert!(output.contains("2024-01-15"), "date in preamble");
        assert!(
            output.contains("simple-theme"),
            "touying simple-theme applied"
        );
        assert!(
            output.contains("#title-slide["),
            "cover uses touying title-slide"
        );
        assert!(output.contains("codly-init"), "codly initialised");
    }

    fn cover_slide(source: &str) -> Slide {
        Slide {
            kind: SlideKind::Cover,
            title: Content::text("My Presentation"),
            body_source: Some(source.to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn test_every_page_carries_a_slide_marker() {
        // `toboggan pdf` queries these to say which slide overflowed. A block
        // without them is a page the report cannot account for.
        let mut talk = make_talk("Talk");
        talk.slides.push(cover_slide("# Talk\n"));
        talk.slides.push(Slide {
            kind: SlideKind::Part,
            title: Content::text("Part One"),
            source_path: Some("01-part/_part.md".into()),
            ..Default::default()
        });
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("One"),
            body_source: Some("# One\n\nBody.\n".to_owned()),
            source_path: Some("01-part/1-one.md".into()),
            ..Default::default()
        });
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        assert_eq!(
            output.matches(r#"at: "start""#).count(),
            3,
            "one start marker per page-producing block, got: {output}"
        );
        assert_eq!(
            output.matches(r#"at: "end""#).count(),
            3,
            "and one end marker each, got: {output}"
        );
        assert!(
            output
                .contains(r#"#metadata((slide: "01-part/1-one.md", at: "start"))<toboggan-slide>"#),
            "a marker names the slide's own file, got: {output}"
        );
    }

    #[test]
    fn test_deck_preamble_replaces_the_generated_one() {
        let mut talk = make_talk("Talk");
        // A *correct* replacement, because this is the example a deck author
        // copies. `header: none` is not decoration: the body emits its own
        // `== <title>` inside `#slide[..]`, and a metropolis theme left to show
        // the current heading prints every title a second time above it — the
        // bug `subslide-preamble: none` fixes for `themes.simple`, which
        // metropolis ignores.
        talk.typst_preamble = Some(
            "#import \"@preview/touying:0.7.3\": *\n#import themes.metropolis: *\n\
             #import \"@preview/codly:1.3.0\": *\n\
             #import \"@preview/codly-languages:0.1.1\": *\n\
             #show: metropolis-theme.with(aspect-ratio: \"4-3\", header: none)\n\
             #show: codly-init.with()\n#codly(languages: codly-languages)"
                .to_owned(),
        );
        talk.slides
            .push(slide_with_source("# Demo\n\nBody text.\n"));
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        assert!(
            output.contains("metropolis-theme"),
            "the deck's own preamble is emitted, got: {output}"
        );
        assert!(
            !output.contains("simple-theme"),
            "the generated preamble is replaced, not appended to, got: {output}"
        );
        assert!(
            output.contains("#title-slide[") && output.contains("#slide["),
            "everything below the preamble is unchanged, got: {output}"
        );
    }

    #[test]
    fn test_cover_body_is_rendered_not_dropped() {
        // Regression: the Cover arm of `write_slide` did nothing, on the grounds
        // that the title and date were emitted by the title slide. Everything
        // else in `_cover.md` — an illustration, a subtitle, a byline — went
        // with them, silently.
        let mut talk = make_talk("My Presentation");
        talk.slides.push(cover_slide(
            "# My Presentation\n\n![The cover](/public/img/cover.png)\n\nBy someone\n",
        ));
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        let title_slide = output
            .split_once("#title-slide[")
            .expect("title slide emitted")
            .1
            .split_once("\n]")
            .expect("title slide closed")
            .0;

        assert!(
            title_slide.contains(r#"#image("/public/img/cover.png")"#),
            "cover illustration rendered inside the title slide, got: {output}"
        );
        assert!(
            title_slide.contains("By someone"),
            "cover prose rendered inside the title slide, got: {output}"
        );
        assert!(
            !title_slide.is_empty() && !title_slide.contains("#slide["),
            "cover stays one page, got: {output}"
        );
        assert!(
            !title_slide.contains("= My Presentation"),
            "the cover's own H1 is stripped, not repeated under the title, got: {output}"
        );
    }

    #[test]
    fn test_empty_cover_leaves_the_title_slide_alone() {
        let mut talk = make_talk("My Presentation");
        talk.slides.push(cover_slide("# My Presentation\n"));
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        let after_date = output
            .split_once("#text(size: 0.8em, fill: gray)[2024-01-15]")
            .expect("date emitted")
            .1
            .split_once("\n]")
            .expect("title slide closed")
            .0;

        assert!(
            after_date
                .lines()
                .all(|line| line.trim().is_empty() || line.contains("#metadata((slide:")),
            "a bodyless cover adds nothing under the date, got: {output}"
        );
    }

    #[test]
    fn test_theme_preamble_does_not_repeat_the_title() {
        // Regression: the theme's `subslide-preamble` displays the current
        // level-2 heading, which is the `== {title}` the slide body already
        // emits, so every content slide printed its title twice.
        let mut talk = make_talk("Talk");
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("Le constat"),
            body_source: Some("# Le constat\n\nBody text.\n".to_owned()),
            // A real slide knows its file, so the page marker names the file
            // rather than falling back to the title.
            source_path: Some("1-constat.md".into()),
            ..Default::default()
        });
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        assert!(
            output.contains("subslide-preamble: none"),
            "theme preamble disabled, got: {output}"
        );
        assert_eq!(
            output.matches("Le constat").count(),
            1,
            "title emitted exactly once, got: {output}"
        );
    }

    #[test]
    fn test_standard_slide_uses_touying_slide() {
        let mut talk = make_talk("Talk");
        talk.slides
            .push(slide_with_source("# Heading\n\nBody text.\n"));
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");
        assert!(
            output.contains("#slide["),
            "standard slide wrapped in #slide[..]"
        );
        assert!(output.contains("Body text"), "body present");
    }

    #[test]
    fn test_heading_inside_body_does_not_trigger_new_section() {
        // Regression: with touying simple-theme, a top-level `= H1` would trigger
        // a new-section-slide. Body markdown of `# Demo` therefore must not be
        // emitted as a bare `= Demo` at document scope — the leading H1 must be
        // consumed as the slide title (rendered via `==` inside #slide[..]).
        let mut talk = make_talk("Talk");
        talk.slides
            .push(slide_with_source("# Demo\n\nFirst body.\n"));
        talk.slides
            .push(slide_with_source("# Other\n\nSecond body.\n"));
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        // Two #slide[..] blocks and zero stray top-level `= ..` headings that
        // would cause touying to inject extra section-slide pages.
        assert_eq!(
            output.matches("#slide[").count(),
            2,
            "expected exactly two standard slides, got: {output}"
        );
        for line in output.lines() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with("= "),
                "top-level `= heading` would trigger an unintended touying section-slide; \
                 offending line: {line}"
            );
        }
    }

    #[test]
    fn test_standard_slide_with_code_block() {
        let mut talk = make_talk("Code Talk");
        let source = "# Demo\n\n```rust\nfn main() {}\n```\n";
        talk.slides.push(slide_with_source(source));

        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(output.contains("```rust"), "fenced code block");
        assert!(output.contains("fn main()"), "code content");
    }

    /// Math goes through `MiTeX`, not Typst's own `$…$`. Typst math is a
    /// different language — `\frac{a}{b}` there resolves `rac` as a variable —
    /// so emitting the LaTeX between bare dollars produced a `.typ` that did
    /// not compile at all.
    #[test]
    fn math_is_rendered_through_mitex() {
        let mut talk = make_talk("Math Talk");
        let source = r"# Math

Inline $x^2$ then

$$x = \frac{-b}{2a}$$
";
        talk.slides.push(slide_with_source(source));

        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(
            output.contains(r#"#import "@preview/mitex:0.2.7": mi, mitex"#),
            "mitex not imported: {output}"
        );
        assert!(output.contains("#mi(`x^2`)"), "inline math: {output}");
        assert!(
            output.contains(r"#mitex(`x = \frac{-b}{2a}`)"),
            "display math: {output}"
        );
    }

    /// `MiTeX` takes its LaTeX as a backtick-delimited raw span, so a literal
    /// backtick cannot be passed through. Falling back to showing the source
    /// keeps the document compiling.
    #[test]
    fn math_containing_a_backtick_degrades_instead_of_breaking_the_document() {
        let mut talk = make_talk("Quoting");
        talk.slides
            .push(slide_with_source("# Q\n\nQuote $x = `y$ here\n"));

        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(output.contains(r#"#raw("x = `y")"#), "fallback: {output}");
        assert!(!output.contains("#mi(`"), "must not emit a broken raw span");
    }

    #[test]
    fn test_slide_with_terminals_emits_placeholder() {
        let mut talk = make_talk("Demo Talk");
        let slide = Slide {
            kind: SlideKind::Standard,
            title: Content::text("Live Demo"),
            terminals: vec![TerminalConfig::new(".")],
            ..Default::default()
        };
        talk.slides.push(slide);

        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(
            output.contains("Live terminal demo"),
            "placeholder present for unmarked live slide"
        );
    }

    #[test]
    fn test_hidden_in_pdf_slide_is_omitted() {
        let mut talk = make_talk("Demo Talk");
        let live_slide = Slide {
            kind: SlideKind::Standard,
            title: Content::text("Live Demo"),
            terminals: vec![TerminalConfig::new(".")],
            hidden_in: BTreeSet::from([RenderTarget::Pdf]),
            ..Default::default()
        };
        let static_slide = Slide {
            kind: SlideKind::Standard,
            title: Content::text("Static Demo"),
            body_source: Some("# Static Demo\n\n```rust\nfn main() {}\n```\n".to_owned()),
            hidden_in: BTreeSet::from([RenderTarget::Web]),
            ..Default::default()
        };
        talk.slides.push(live_slide);
        talk.slides.push(static_slide);

        let filtered_slides: Vec<_> = talk
            .slides
            .iter()
            .filter(|slide| !slide.is_hidden_from(RenderTarget::Pdf))
            .cloned()
            .collect();
        let mut filtered_talk = talk.clone();
        filtered_talk.slides = filtered_slides;

        let bytes = generate_typst(&filtered_talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(
            !output.contains("Live Demo"),
            "pdf-hidden slide must be absent"
        );
        assert!(
            output.contains("Static Demo"),
            "web-hidden slide must be present"
        );
    }

    #[test]
    fn test_escape_typst() {
        assert_eq!(escape_typst("hello"), "hello");
        assert_eq!(escape_typst("a#b"), r"a\#b");
        assert_eq!(escape_typst("x[y]"), r"x\[y\]");
        assert_eq!(escape_typst("a*b_c"), r"a\*b\_c");
    }

    #[test]
    fn test_escape_typst_string_url() {
        // A URL with `"` must not break the Typst string literal.
        assert_eq!(
            escape_typst_string(r#"https://x.com/?a="b""#),
            r#"https://x.com/?a=\"b\""#
        );
        assert_eq!(escape_typst_string(r"path\to"), r"path\\to");
        assert_eq!(escape_typst_string("plain"), "plain");
    }

    #[test]
    fn test_link_with_special_url_characters() {
        // A double-quote inside a URL must be escaped so typst compile succeeds.
        let result = md_to_typst(r#"[text](https://x.com/?a="b")"#);
        assert!(
            result.contains(r#"#link("https://x.com/?a=\"b\"")"#),
            "url escaped in link"
        );
    }

    #[test]
    fn test_image_url_is_escaped() {
        // Same `"` quote-in-URL hazard applies to images — pin it so we don't
        // regress on the symmetric arm.
        let result = md_to_typst(r#"![alt](https://x.com/a"b.png)"#);
        assert!(
            result.contains(r#"#image("https://x.com/a\"b.png")"#),
            "image url escaped, got: {result}"
        );
    }

    #[test]
    fn test_md_to_typst_headings() {
        assert!(md_to_typst("# H1\n").contains("= H1"), "h1");
        assert!(md_to_typst("## H2\n").contains("== H2"), "h2");
        assert!(md_to_typst("### H3\n").contains("=== H3"), "h3");
    }

    #[test]
    fn test_md_to_typst_unordered_list() {
        let result = md_to_typst("- alpha\n- beta\n- gamma\n");
        assert!(result.contains("- alpha"), "first item");
        assert!(result.contains("- beta"), "second item");
        assert!(result.contains("- gamma"), "third item");
    }

    #[test]
    fn test_md_to_typst_ordered_list() {
        let result = md_to_typst("1. first\n2. second\n");
        assert!(result.contains("+ first"), "first ordered item");
        assert!(result.contains("+ second"), "second ordered item");
    }

    #[test]
    fn test_md_to_typst_nested_list() {
        let result = md_to_typst("- outer\n  - inner\n");
        assert!(result.contains("- outer"), "outer item");
        assert!(result.contains("  - inner"), "nested item indented");
    }

    #[test]
    fn test_md_to_typst_table() {
        let result = md_to_typst("| A | B |\n|---|---|\n| 1 | 2 |\n");
        assert!(result.contains("#table("), "table macro");
        assert!(result.contains("columns: 2"), "column count");
        assert!(result.contains("[A]"), "header cell A");
        assert!(result.contains("[1]"), "data cell 1");
    }

    #[test]
    fn test_md_to_typst_empty_table_body() {
        // Degenerate input (single empty header cell after trim) must never emit
        // `columns: 0` — that is invalid Typst. A 1-column empty cell is fine.
        let result = md_to_typst("| |\n|---|\n");
        assert!(
            !result.contains("columns: 0"),
            "columns: 0 must never be emitted, got: {result}"
        );
    }

    #[test]
    fn test_md_to_typst_zero_column_table_is_skipped() {
        // A header row that parses to zero cells must skip the #table call
        // entirely (we can't synthesise the cell count out of thin air).
        // Comrak rarely emits this from markdown, so verify via direct call:
        // an empty header row would imply columns: 0 which the renderer drops.
        let result = md_to_typst("");
        assert!(
            !result.contains("columns: 0") && !result.contains("#table("),
            "empty input never emits a table, got: {result}"
        );
    }

    #[test]
    fn test_md_to_typst_table_pads_short_rows() {
        // Header has 3 columns, body row only 2 cells. Renderer pads with empty
        // cells so the emitted #table is well-formed.
        let result = md_to_typst("| A | B | C |\n|---|---|---|\n| 1 | 2 |\n");
        assert!(result.contains("columns: 3"), "3-column declaration");
        assert!(result.contains("[1]"), "first data cell present");
        assert!(result.contains("[2]"), "second data cell present");
        assert!(
            result.contains("[],"),
            "missing third cell padded with empty"
        );
    }

    #[test]
    fn test_empty_body_slide_emits_visible_placeholder() {
        // Regression: previously, an empty body silently dropped the page from
        // the PDF. Now it emits a visible red placeholder so the gap is obvious.
        let mut talk = make_talk("Talk");
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("Ghost Slide"),
            body_source: Some(String::new()),
            ..Default::default()
        });
        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");
        assert!(output.contains("Ghost Slide"), "slide title still emitted");
        assert!(
            output.contains("Empty slide"),
            "visible placeholder present, got: {output}"
        );
    }

    #[test]
    fn test_html_without_alt_emits_visible_placeholder() {
        let html_content = Content::Html {
            raw: "<p>raw html</p>".to_owned(),
            style: Style::default(),
            alt: None,
        };
        let result = content_to_typst(&html_content);
        assert!(
            result.contains("HTML content not exportable"),
            "visible placeholder for HTML-without-alt, got: {result}"
        );
        assert!(
            !result.contains("raw html"),
            "raw HTML content must not leak into Typst output"
        );
    }

    #[test]
    fn test_md_to_typst_code_block() {
        let result = md_to_typst("```rust\nlet x = 1;\n```\n");
        assert!(result.contains("```rust"), "fenced code block preserved");
        assert!(result.contains("let x = 1;"), "code content preserved");
    }

    /// Typst has no equivalent of `CommonMark`'s "more backticks open a longer
    /// span" rule: ``` `` ``` is an empty raw span, so the delimiter closes at
    /// once and the literal backtick after it opens a span that runs away to
    /// the end of the file. This test used to assert that broken output.
    ///
    /// It cost the guide deck its PDF: a keyboard table with a backtick in one
    /// cell desynchronised every fence after it, and `typst` reported the
    /// failure ninety lines later, on a `$` in an unrelated shell transcript.
    /// The same defect as the inline case, one function down, and the deck that
    /// triggers it is the guide: documenting markdown means a fence inside a
    /// fence. Typst reads one or three backticks and nothing else, so padding
    /// the fence out to four closed the block at the third and spilled the rest
    /// into the document as markup.
    #[test]
    fn a_fenced_block_containing_a_fence_is_emitted_as_raw() {
        let result = md_to_typst("````markdown\n```rust\nlet x = 1;\n```\n````\n");
        assert!(
            result.contains("#raw(block: true"),
            "backtick content goes through #raw: {result}"
        );
        assert!(
            !result.contains("````"),
            "no four-backtick fence, which Typst does not read: {result}"
        );
        assert!(
            result.contains(r"let x = 1;"),
            "the code survives: {result}"
        );
    }

    /// A code block reaches `#raw` as a string literal, so its own line breaks
    /// have to be escaped or they end the literal.
    /// A Mermaid fence becomes an embedded SVG rather than a code block, so the
    /// PDF shows the diagram the web client shows.
    #[test]
    fn a_mermaid_fence_becomes_an_embedded_svg() {
        let output = md_to_typst("```mermaid\nflowchart LR\n  A --> B\n```\n");
        assert!(
            output.contains(r#"image(diagram, format: "svg""#),
            "diagram not embedded: {output}"
        );
        assert!(
            output.contains("calc.min(natural, available.width)"),
            "diagram not clamped to the available width: {output}"
        );
        assert!(!output.contains("```"), "fence left as code: {output}");
    }

    /// An explicit `width=` wins over the natural-size clamp, and is the same
    /// value the HTML wrapper carries.
    #[test]
    fn a_mermaid_fence_honours_an_explicit_width() {
        let output = md_to_typst("```mermaid:width=60%\nflowchart LR\n  A --> B\n```\n");
        assert!(output.contains("width: 60%"), "{output}");
        assert!(!output.contains("calc.min"), "{output}");
    }

    /// `alt=` is the author's description of the picture, and Typst carries it
    /// into the PDF's accessibility tree — so a label written once serves the
    /// screen reader on the web and the one reading the handout.
    #[test]
    fn a_mermaid_fence_carries_its_alt_text_into_the_pdf() {
        let sized = md_to_typst(
            "```mermaid:width=60%,alt=Write then build then present\nflowchart LR\n  A --> B\n```\n",
        );
        assert!(
            sized.contains(r#"alt: "Write then build then present""#),
            "{sized}"
        );

        let natural =
            md_to_typst("```mermaid:alt=A \"quoted\" label\nflowchart LR\n  A --> B\n```\n");
        assert!(natural.contains("calc.min"), "{natural}");
        assert!(
            natural.contains(r#"alt: "A \"quoted\" label""#),
            "alt not escaped: {natural}"
        );
    }

    #[test]
    fn a_mermaid_fence_without_alt_emits_no_alt_argument() {
        let output = md_to_typst("```mermaid\nflowchart LR\n  A --> B\n```\n");
        assert!(!output.contains("alt:"), "{output}");
    }

    /// The SVG lands inside a Typst string literal, so its quotes and
    /// backslashes have to be escaped or the whole compile fails — the same
    /// class of bug as the backtick that once cost the guide its PDF.
    #[test]
    fn an_embedded_diagram_escapes_its_quotes() {
        let output = md_to_typst("```mermaid\nflowchart LR\n  A --> B\n```\n");
        assert!(
            output.contains(r#"bytes("<svg xmlns=\"http:"#),
            "svg quotes not escaped: {output}"
        );
    }

    #[test]
    fn a_raw_code_block_escapes_its_line_breaks() {
        let result = md_to_typst("````text\n``\nsecond line\n````\n");
        assert!(
            result.contains(r"\n"),
            "line breaks escaped inside the string literal: {result}"
        );
    }

    /// An alert title is the author's text and lands inside a Typst string
    /// literal. A quote in it closed the argument early and failed the whole
    /// compile — the same class as the backtick below, on an adjacent line.
    #[test]
    fn an_alert_title_containing_a_quote_is_escaped() {
        let result = md_to_typst("> [!NOTE] He said \"hi\"\n> Body.\n");
        assert!(
            result.contains(r#"\"hi\""#),
            "the quote is escaped inside the title argument: {result}"
        );
        assert!(
            !result.contains(r#"title: "He said "hi""#),
            "the unescaped form would not compile: {result}"
        );
    }

    #[test]
    fn inline_code_containing_a_backtick_is_emitted_as_raw() {
        let result = md_to_typst("Use `` `tick` `` here.");
        assert!(
            result.contains(r#"#raw("`tick`")"#),
            "backtick content goes through #raw: {result}"
        );
        assert!(
            !result.contains("`` "),
            "no multi-backtick inline delimiter: {result}"
        );
    }

    #[test]
    fn test_inline_code_plain() {
        let result = md_to_typst("Use `foo` here.");
        assert!(
            result.contains("`foo`"),
            "single-backtick for plain inline code"
        );
    }

    #[test]
    fn test_section_slide() {
        let mut talk = make_talk("Talk");
        let slide = Slide {
            kind: SlideKind::Part,
            title: Content::text("Part One"),
            ..Default::default()
        };
        talk.slides.push(slide);

        let bytes = generate_typst(&talk, &MermaidRenderer::default()).expect("render");
        let output = String::from_utf8(bytes).expect("utf8");

        assert!(output.contains("Part One"), "section title present");
    }

    #[test]
    fn test_part_slide_body_is_preserved() {
        // Regression: Part slides loaded from `_part.md` carry prose in body_source
        // (e.g. examples/riir-folder/02-success-stories/_part.md). The Typst section
        // writer was discarding the body — only the title was emitted.
        let mut talk = make_talk("Talk");
        let slide = Slide {
            kind: SlideKind::Part,
            title: Content::text("Success Stories"),
            body_source: Some("# Success Stories\n\nIntro prose for the part.".to_owned()),
            ..Default::default()
        };
        talk.slides.push(slide);

        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");

        assert!(output.contains("Success Stories"), "part title present");
        assert!(
            output.contains("Intro prose for the part"),
            "part body preserved in Typst output"
        );
    }

    #[test]
    fn test_gfm_alert_note_renders_as_callout() {
        let result = md_to_typst("> [!NOTE]\n> Useful information.\n");
        assert!(
            result.contains("Useful information"),
            "alert body preserved"
        );
        assert!(
            result.contains("Note") || result.contains("NOTE") || result.contains("#info"),
            "NOTE label or info clue surfaced in Typst output, got: {result}"
        );
    }

    #[test]
    fn test_gfm_alert_tip_renders_as_callout() {
        let result = md_to_typst("> [!TIP]\n> Useful tip.\n");
        assert!(result.contains("Useful tip"), "tip body preserved");
        assert!(
            result.contains("Tip") || result.contains("TIP") || result.contains("#tip"),
            "TIP label or tip clue surfaced, got: {result}"
        );
    }

    #[test]
    fn test_gfm_alert_important_renders_as_callout() {
        let result = md_to_typst("> [!IMPORTANT]\n> Critical info.\n");
        assert!(result.contains("Critical info"), "important body preserved");
        assert!(
            result.contains("Important") || result.contains("IMPORTANT"),
            "IMPORTANT label surfaced, got: {result}"
        );
    }

    #[test]
    fn test_gfm_alert_warning_renders_as_callout() {
        let result = md_to_typst("> [!WARNING]\n> Beware.\n");
        assert!(result.contains("Beware"), "warning body preserved");
        assert!(
            result.contains("Warning") || result.contains("WARNING") || result.contains("#warning"),
            "WARNING label or warning clue surfaced, got: {result}"
        );
    }

    #[test]
    fn test_gfm_alert_caution_renders_as_callout() {
        let result = md_to_typst("> [!CAUTION]\n> Danger ahead.\n");
        assert!(result.contains("Danger ahead"), "caution body preserved");
        assert!(
            result.contains("Caution") || result.contains("CAUTION"),
            "CAUTION label surfaced, got: {result}"
        );
    }

    #[test]
    fn test_slide_with_style_classes_produces_content() {
        // Style classes are intentionally not rendered in Typst output.
        // Verify the slide body is still present even when classes are set.
        let mut talk = make_talk("Talk");
        let slide = Slide {
            kind: SlideKind::Standard,
            style: Style {
                classes: vec!["wide".to_owned()],
                style: None,
            },
            body_source: Some("# Styled\n\nContent.".to_owned()),
            ..Default::default()
        };
        talk.slides.push(slide);

        let output =
            String::from_utf8(generate_typst(&talk, &MermaidRenderer::default()).expect("render"))
                .expect("utf8");
        assert!(
            output.contains("Content."),
            "slide body present despite style classes"
        );
        assert!(
            !output.contains("wide"),
            "style class not emitted in Typst output"
        );
    }
}
