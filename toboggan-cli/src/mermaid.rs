//! Renders ` ```mermaid ` fences to SVG while the deck builds.
//!
//! This mirrors what the parser already does for `$…$` math: the conversion
//! happens here rather than in the browser, so an exported deck draws its
//! diagrams with no script, no CDN and no web font, and a broken diagram fails
//! the one-shot commands and names the file instead of rendering as nothing in
//! front of an audience. The live-reload watcher is the deliberate exception:
//! it drops the slide and keeps serving, because tearing down a running server
//! over a half-written diagram is worse mid-rehearsal.
//!
//! Two independent pipelines consume the same fence — [`crate::parser`] for the
//! HTML the web client and the HTML export read, and the Typst renderer behind
//! [`crate::output`] for the PDF and the thumbnails — so both go through
//! [`MermaidRenderer`] and cannot disagree about what a fence means, *given the
//! same settings*. Handing them different ones is still possible: `serve` takes
//! a prebuilt talk and needs `--mermaid-config` to redraw it the way it was
//! drawn originally.

use std::fmt::Write as _;
use std::path::Path;

use mermaid_rs_renderer::{ParseError, RenderOptions, Theme, render_strict};
use miette::SourceSpan;

use crate::error::{Result, TobogganCliError};
use crate::output::html::escape_html;

/// The fence language that marks a Mermaid diagram.
const FENCE_LANG: &str = "mermaid";

/// The theme names [`Theme::from_name`] accepts, for the two places that reject
/// one: a fence's `theme=` and the deck's config file. Naming them once keeps
/// the two surfaces from disagreeing about what is spellable.
const THEMES: &str = "default, base, mermaid, dark, forest, neutral or modern";

/// The class placed on the `<div>` wrapping a rendered diagram.
pub const WRAPPER_CLASS: &str = "mermaid";

/// Added when the fence gave an explicit `width=`, which makes the diagram fill
/// the wrapper rather than keep its natural size. Without it the SVG's own pixel
/// width would win and a `width=` that asked to *enlarge* a small diagram would
/// do nothing — which is not what the same fence does in the PDF.
const SIZED_CLASS: &str = "mermaid-sized";

/// Deck-level Mermaid settings, resolved once per build.
///
/// Built from a JSON file in Mermaid's own config shape (`theme`,
/// `themeVariables`, `preferredAspectRatio`, `flowchart: { nodeSpacing, … }`),
/// which per-fence parameters then override.
#[derive(Debug, Clone)]
pub struct MermaidRenderer {
    base: RenderOptions,
}

impl Default for MermaidRenderer {
    fn default() -> Self {
        Self {
            base: default_options(),
        }
    }
}

/// The renderer's own defaults, before any config file or fence parameter.
///
/// `fast_text_metrics` is on because the alternative measures labels against
/// the *building machine's* installed fonts, which would make a deck's geometry
/// — and therefore the built `.toml` artifacts committed to this repo — differ
/// between machines. The fast path uses calibrated widths and skips the font
/// database for a flowchart's ASCII labels, which is what this repo's own decks
/// are made of.
///
/// It is not a blanket guarantee. A non-ASCII label falls through to the font
/// database, and so do several diagram kinds the flag never reaches — pie
/// percentages and class/ER attribute columns measure text through code that
/// does not consult it at all (`render.rs` and `pie.rs` in
/// `mermaid_rs_renderer 0.3.1`). A deck using those is not byte-reproducible.
fn default_options() -> RenderOptions {
    let mut options = RenderOptions::default();
    options.layout.fast_text_metrics = true;
    options
}

impl MermaidRenderer {
    /// Loads deck-level defaults from a Mermaid JSON config file.
    ///
    /// `None` keeps the renderer's own defaults, i.e. [`Self::default`].
    ///
    /// # Errors
    /// Returns [`TobogganCliError::MermaidConfig`] if the file cannot be read
    /// or is not valid Mermaid configuration JSON.
    pub fn from_config(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let declared = validate_config(path)?;
        let config = mermaid_rs_renderer::config::load_config(Some(path)).map_err(|err| {
            TobogganCliError::MermaidConfig {
                path: path.to_path_buf(),
                message: err.to_string(),
            }
        })?;
        // `load_config` starts from the *crate's* defaults and folds "absent"
        // into "set to the default", so copying its result wholesale would
        // quietly undo both of ours: the theme would drop from `modern` to
        // `default`, and `fast_text_metrics` back to `false` — making every
        // deck that merely retints its diagrams build differently per machine.
        //
        // Asking the file which top-level keys it names does not work either:
        // `themeVariables` is folded *into* `config.theme`, so a config that
        // only retints — the common case — names no `theme` key and would have
        // every variable dropped. Compare against the crate's own baseline
        // instead, which sees a change wherever it actually happened.
        let baseline = mermaid_rs_renderer::config::load_config(None).map_err(|err| {
            TobogganCliError::MermaidConfig {
                path: path.to_path_buf(),
                message: format!("could not read the renderer's own defaults: {err}"),
            }
        })?;
        let mut base = default_options();
        if differs(&config.theme, &baseline.theme) {
            base.theme = config.theme;
        }
        if differs(&config.layout, &baseline.layout) {
            base.layout = config.layout;
        }
        // `fast_text_metrics` is the one setting where our default is the
        // *opposite* of the crate's, so a value comparison cannot tell "the
        // author asked for false" from "the author said nothing". Only here
        // does the declared key still decide.
        base.layout.fast_text_metrics = match declared.get("fastTextMetrics") {
            Some(serde_json::Value::Bool(asked)) => *asked,
            _ => default_options().layout.fast_text_metrics,
        };
        Ok(Self { base })
    }

    /// Parses a fence info string.
    ///
    /// Returns `None` when the fence is not a Mermaid one, so a caller can fall
    /// through to its normal code-block handling.
    ///
    /// # Errors
    /// Returns [`TobogganCliError::InvalidMermaidFence`] for a malformed or
    /// unknown parameter, so a typo is reported rather than silently ignored.
    #[must_use]
    pub fn parse_info(&self, info: &str, source_name: &str) -> Option<Result<MermaidFence>> {
        let info = info.trim();
        // Step over the separator by its own width: `char::is_whitespace` is
        // true for NBSP and U+2028, and a fixed `+ 1` would land mid-character.
        // The slice would then fail, and a failed slice here reads to every
        // caller as "not a Mermaid fence" — so a pasted deck would render its
        // diagram as a code block, silently, in front of an audience.
        let separator = info
            .char_indices()
            .find(|(_, chr)| *chr == ':' || chr.is_whitespace());
        let (lang, params) = match separator {
            Some((index, chr)) => (
                info.get(..index).unwrap_or_default(),
                info.get(index + chr.len_utf8()..).unwrap_or_default(),
            ),
            None => (info, ""),
        };
        if lang != FENCE_LANG {
            return None;
        }
        Some(self.parse_params(params.trim(), source_name))
    }

    fn parse_params(&self, params: &str, source_name: &str) -> Result<MermaidFence> {
        let mut fence = MermaidFence {
            options: self.base.clone(),
            background: Background::Transparent,
            width: None,
            class: None,
            alt: None,
        };
        let parts = split_params(params)
            .map_err(|reason| fence_error(source_name, params, params, &reason))?;
        // `alt=` runs to the end of the info string. An accessible label is a
        // sentence and a sentence has commas in it, so splitting on them
        // rejected `alt=Write, then build` with "parameter `then build` is not
        // `key=value`" — an error naming a fragment the author never wrote.
        let alt_at = parts
            .iter()
            .position(|part| part.trim_start().starts_with("alt="));
        let (head, alt) = match alt_at {
            Some(index) => (
                parts.get(..index).unwrap_or_default(),
                parts.get(index..).map(|rest| rest.join(",")),
            ),
            None => (parts.as_slice(), None),
        };
        for param in head
            .iter()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
        {
            let (key, value) = param.split_once('=').ok_or_else(|| {
                fence_error(
                    source_name,
                    params,
                    param,
                    &format!("parameter `{param}` is not `key=value`"),
                )
            })?;
            fence
                .apply(key.trim(), value.trim())
                .map_err(|reason| fence_error(source_name, params, param, &reason))?;
        }
        if let Some((index, alt)) = alt_at.zip(alt) {
            let label = alt.trim_start().trim_start_matches("alt=").trim();
            // The label was rejoined from several parts, so span from where it
            // started to the end of the fence rather than from the copy.
            let spanned = parts.get(index).map_or(params, |part| {
                params.get(offset_within(params, part)..).unwrap_or(params)
            });
            fence
                .apply("alt", label)
                .map_err(|reason| fence_error(source_name, params, spanned, &reason))?;
        }
        Ok(fence)
    }

    /// Renders a diagram to a standalone `<svg …>` element.
    ///
    /// The element's ids are namespaced, so several diagrams can share one
    /// document — which they do in the HTML export, where every slide lands in
    /// the same page.
    ///
    /// # Errors
    /// Returns [`TobogganCliError::InvalidMermaid`] if the diagram does not parse.
    pub fn render_svg(
        &self,
        fence: &MermaidFence,
        diagram: &str,
        source_name: &str,
    ) -> Result<String> {
        let svg = render_strict(diagram, fence.options.clone()).map_err(|err| {
            TobogganCliError::invalid_mermaid(
                source_name,
                diagram.to_owned(),
                error_span(&err, diagram),
                err.to_string(),
            )
        })?;
        let svg = fence.background.apply(&svg);
        Ok(namespace_ids(&svg))
    }

    /// Renders a diagram as the HTML block that replaces the fence.
    ///
    /// # Errors
    /// See [`Self::render_svg`].
    pub fn render_html(
        &self,
        fence: &MermaidFence,
        diagram: &str,
        source_name: &str,
    ) -> Result<String> {
        let svg = self.render_svg(fence, diagram, source_name)?;

        let mut open = format!(r#"<div class="{WRAPPER_CLASS}"#);
        if fence.width.is_some() {
            let _ = write!(open, " {SIZED_CLASS}");
        }
        if let Some(class) = &fence.class {
            open.push(' ');
            open.push_str(&escape_html(class));
        }
        open.push('"');
        // `Length` renders as a number and one of six known units, so unlike
        // `class` and `alt` it needs no escaping to be safe here.
        if let Some(width) = fence.width {
            let _ = write!(open, r#" style="width: {width}""#);
        }
        if let Some(alt) = &fence.alt {
            let _ = write!(open, r#" role="img" aria-label="{}""#, escape_html(alt));
        }
        open.push('>');

        Ok(format!("{open}{svg}</div>\n"))
    }
}

/// A Mermaid fence's resolved parameters.
#[derive(Debug, Clone)]
pub struct MermaidFence {
    options: RenderOptions,
    background: Background,
    /// Explicit size for the rendered diagram.
    width: Option<Length>,
    /// Extra classes for the HTML wrapper.
    ///
    /// HTML only: Typst has no notion to map a CSS class onto, so the PDF
    /// ignores it. Unlike `width` and `alt`, which both pipelines honour.
    class: Option<String>,
    /// Accessible label, in the HTML wrapper and in the PDF alike.
    ///
    /// Written last in a fence, because it runs to the end of the info string
    /// — see `parse_params`.
    alt: Option<String>,
}

impl MermaidFence {
    /// The explicit size the author asked for.
    #[must_use]
    pub fn width(&self) -> Option<Length> {
        self.width
    }

    /// The author's description of the diagram, if they gave one.
    #[must_use]
    pub fn alt(&self) -> Option<&str> {
        self.alt.as_deref()
    }

    /// Applies one `key=value` parameter, reporting the reason it was rejected.
    fn apply(&mut self, key: &str, value: &str) -> std::result::Result<(), String> {
        match key {
            "theme" => {
                self.options.theme = Theme::from_name(value)
                    .ok_or_else(|| format!("unknown theme `{value}` (expected {THEMES})"))?;
            }
            "background" => self.background = Background::parse(value)?,
            "nodeSpacing" => self.options.layout.node_spacing = parse_positive(key, value)?,
            "rankSpacing" => self.options.layout.rank_spacing = parse_positive(key, value)?,
            "maxLabelWidth" => {
                self.options.layout.max_label_width_chars = parse_number(key, value)?;
            }
            "aspectRatio" => {
                self.options.layout.preferred_aspect_ratio = Some(parse_aspect_ratio(value)?);
            }
            "fastText" => self.options.layout.fast_text_metrics = parse_bool(key, value)?,
            "width" => self.width = Some(Length::parse(value)?),
            "class" => self.class = Some(value.to_owned()),
            "alt" => self.alt = Some(value.to_owned()),
            other => {
                return Err(format!(
                    "unknown parameter `{other}` (expected theme, background, nodeSpacing, \
                     rankSpacing, maxLabelWidth, aspectRatio, fastText, width, class or alt)"
                ));
            }
        }
        Ok(())
    }
}

/// An explicit diagram width, in a unit both CSS and Typst understand.
///
/// Parsed rather than carried as the author's own string, because the same
/// value is written into two different languages: a CSS `style` attribute for
/// the web, and generated Typst source for the PDF. A raw string would let
/// `width=200px` — valid CSS, not a Typst length — build a deck that renders on
/// the projector and fails `toboggan pdf`, and would let a fence close the
/// `image(…)` call it is interpolated into and inject arbitrary Typst.
///
/// The two spellings coincide, so one [`Display`](std::fmt::Display) serves
/// both; the point of the type is that only these units can be spelled at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    value: f32,
    unit: Unit,
}

/// The units CSS and Typst share. Notably absent: `px`, `rem`, `vw`, `vh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Percent,
    Pt,
    Mm,
    Cm,
    In,
    Em,
}

const UNITS: &str = "%, pt, mm, cm, in or em";

impl Length {
    fn parse(input: &str) -> std::result::Result<Self, String> {
        let text = input.trim();
        let boundary = text
            .find(|chr: char| !matches!(chr, '0'..='9' | '.' | '+' | '-'))
            .ok_or_else(|| format!("`width` needs a unit ({UNITS}), got `{input}`"))?;
        let (number, unit) = text
            .split_at_checked(boundary)
            .ok_or_else(|| format!("`width` is not a length, got `{input}`"))?;
        let value = number
            .parse::<f32>()
            .map_err(|_| format!("`width` expects a number before the unit, got `{input}`"))?;
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("`width` must be positive, got `{input}`"));
        }
        Ok(Self {
            value,
            unit: Unit::parse(unit)?,
        })
    }
}

impl Unit {
    fn parse(text: &str) -> std::result::Result<Self, String> {
        match text {
            "%" => Ok(Self::Percent),
            "pt" => Ok(Self::Pt),
            "mm" => Ok(Self::Mm),
            "cm" => Ok(Self::Cm),
            "in" => Ok(Self::In),
            "em" => Ok(Self::Em),
            other => Err(format!(
                "`width` unit `{other}` is not shared by CSS and Typst (expected {UNITS})"
            )),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Percent => "%",
            Self::Pt => "pt",
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::In => "in",
            Self::Em => "em",
        }
    }
}

impl std::fmt::Display for Length {
    /// `{}` on an `f32` never uses exponent notation, which is what makes this
    /// safe to interpolate into generated Typst. `{:e}` would emit `3.8e1%`,
    /// which Typst rejects — and every existing test would still pass, because
    /// they all use small round numbers. See
    /// `a_width_never_renders_in_exponent_notation`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}{}", self.value, self.unit.as_str())
    }
}

/// What sits behind the diagram.
///
/// Mermaid paints an opaque page-coloured rectangle over the whole viewBox,
/// which on a themed slide reads as a white box punched into the design. A
/// slide almost always wants the deck's own background showing through, so that
/// rectangle is dropped unless the author asks for it back.
#[derive(Debug, Clone, Default)]
enum Background {
    /// Drop Mermaid's background rectangle.
    #[default]
    Transparent,
    /// Keep whatever the theme paints.
    Theme,
    /// Repaint the rectangle in this colour.
    Color(String),
}

impl Background {
    /// Rejects an unrecognised value rather than taking it for a colour.
    ///
    /// Without the colour check, a misspelled `transparent` would be written
    /// straight into `fill="…"` — an invalid paint, which renders black. That
    /// is a silently wrong slide, where a misspelled `theme` is a build error
    /// naming the file; this parameter should not be the exception.
    fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "transparent" | "none" => Ok(Self::Transparent),
            "theme" | "default" => Ok(Self::Theme),
            color if is_color(color) => Ok(Self::Color(color.to_owned())),
            other => Err(format!(
                "`background` expects transparent, theme or a colour, got `{other}`"
            )),
        }
    }

    /// Drops or repaints Mermaid's background rectangle, leaving the rest of
    /// the document alone.
    fn apply(&self, svg: &str) -> String {
        if matches!(self, Self::Theme) {
            return svg.to_owned();
        }
        let Some((start, end)) = find_background_rect(svg) else {
            return svg.to_owned();
        };
        let mut out = String::with_capacity(svg.len());
        out.push_str(svg.get(..start).unwrap_or_default());
        if let Self::Color(color) = self {
            out.push_str(&replace_fill(
                svg.get(start..end).unwrap_or_default(),
                color,
            ));
        }
        out.push_str(svg.get(end..).unwrap_or_default());
        out
    }
}

/// Locates Mermaid's background rectangle: the `<rect …/>` that immediately
/// follows the root `<svg …>` tag and carries nothing but geometry and a fill.
///
/// The shape is checked rather than assumed, so a future renderer that puts
/// something else there leaves the diagram untouched instead of losing a shape.
fn find_background_rect(svg: &str) -> Option<(usize, usize)> {
    let root_end = svg.find('>')? + 1;
    let rest = svg.get(root_end..)?;
    if !rest.starts_with("<rect ") {
        return None;
    }
    let close = rest.find("/>")? + 2;
    let attributes = rest.get("<rect ".len()..close - 2)?;
    let keys = attributes
        .split_whitespace()
        .filter_map(|attribute| attribute.split_once('='))
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    if keys != ["x", "y", "width", "height", "fill"] {
        return None;
    }
    Some((root_end, root_end + close))
}

/// Rewrites the `fill="…"` of a single element.
fn replace_fill(element: &str, color: &str) -> String {
    let Some(start) = element.find(r#"fill=""#) else {
        return element.to_owned();
    };
    let value_start = start + r#"fill=""#.len();
    let Some(length) = element.get(value_start..).and_then(|it| it.find('"')) else {
        return element.to_owned();
    };
    let mut out = String::with_capacity(element.len());
    out.push_str(element.get(..value_start).unwrap_or_default());
    out.push_str(&escape_html(color));
    out.push_str(element.get(value_start + length..).unwrap_or_default());
    out
}

/// Prefixes every `id` in an SVG, and every reference to one, so that several
/// diagrams can live in the same document without stealing each other's
/// arrowheads.
///
/// The prefix is derived from the SVG's own bytes, which keeps it stable across
/// builds and machines and makes the order diagrams are rendered in irrelevant.
/// Two byte-identical diagrams therefore share a prefix — harmless, because
/// what they would then share is byte-identical too.
fn namespace_ids(svg: &str) -> String {
    let ids = collect_ids(svg);
    if ids.is_empty() {
        return svg.to_owned();
    }
    let prefix = format!("m{:08x}-", fnv1a(svg.as_bytes()));

    let mut out = String::with_capacity(svg.len() + ids.len() * prefix.len() * 2);
    let mut cursor = 0;
    while let Some(rest) = svg.get(cursor..).filter(|rest| !rest.is_empty()) {
        if let Some((consumed, emitted)) =
            rewrite_reference(rest, &ids, &prefix, starts_attribute(svg, cursor))
        {
            out.push_str(&emitted);
            cursor += consumed;
            continue;
        }
        let step = rest.chars().next().map_or(1, char::len_utf8);
        out.push_str(rest.get(..step).unwrap_or_default());
        cursor += step;
    }
    out
}

/// Whether `offset` is where an XML attribute could start, i.e. at the very
/// beginning or straight after whitespace. Without this `data-edge-id="…"` —
/// a label, not an id — would be rewritten along with the real ids.
fn starts_attribute(svg: &str, offset: usize) -> bool {
    offset == 0
        || svg
            .get(..offset)
            .and_then(|before| before.chars().next_back())
            .is_some_and(char::is_whitespace)
}

/// Rewrites one id definition or reference at the start of `rest`.
///
/// Returns how many bytes were consumed and what to emit in their place.
fn rewrite_reference(
    rest: &str,
    ids: &[String],
    prefix: &str,
    at_attribute_start: bool,
) -> Option<(usize, String)> {
    for (opening, closing, needs_boundary) in [
        (r#"id=""#, '"', true),
        ("url(#", ')', false),
        // `xlink:href="#…"` too, hence no attribute-boundary requirement: the
        // `#` already makes this unambiguously a fragment reference.
        (r##"href="#"##, '"', false),
    ] {
        if !rest.starts_with(opening) || (needs_boundary && !at_attribute_start) {
            continue;
        }
        let value = rest.get(opening.len()..)?;
        let length = value.find(closing)?;
        let name = value.get(..length)?;
        if !ids.iter().any(|known| known == name) {
            continue;
        }
        return Some((opening.len() + length, format!("{opening}{prefix}{name}")));
    }
    None
}

/// Every `id="…"` value defined in the document.
///
/// Only an attribute that *starts* a word counts, so `data-edge-id="edge-0"` —
/// a label, not an id — is left alone.
fn collect_ids(svg: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut offset = 0;
    while let Some(found) = svg.get(offset..).and_then(|it| it.find(r#"id=""#)) {
        let start = offset + found;
        let preceded_by_boundary = start == 0
            || svg
                .get(..start)
                .and_then(|it| it.chars().next_back())
                .is_some_and(char::is_whitespace);
        let value_start = start + r#"id=""#.len();
        offset = value_start;
        if !preceded_by_boundary {
            continue;
        }
        let Some(length) = svg.get(value_start..).and_then(|it| it.find('"')) else {
            break;
        };
        if let Some(name) = svg.get(value_start..value_start + length)
            && !name.is_empty()
            && !ids.iter().any(|known| known == name)
        {
            ids.push(name.to_owned());
        }
        offset = value_start + length;
    }
    ids
}

/// FNV-1a, 32 bit. Hand-rolled rather than [`std::hash::DefaultHasher`] so the
/// prefixes stay identical across Rust releases — the built `.toml` artifacts
/// in `examples/` are committed, and a hash that drifts would churn them.
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Where in the diagram to point the caret.
///
/// Every [`ParseError`] carries a 1-based line, and two of them a column. The
/// span runs from there to the end of that line: the crate reports where a
/// token went wrong but not how long it was, and underlining the rest of the
/// line reads better than a bare one-character caret for all four variants —
/// including `UnclosedSubgraph`, which points at the line that opened it.
///
/// `ParseError` is `#[non_exhaustive]`, so a variant added upstream falls back
/// to spanning the whole diagram rather than failing to compile.
fn error_span(error: &ParseError, diagram: &str) -> SourceSpan {
    let position = match error {
        ParseError::UnknownParticipant { line, .. } => Some((*line, 1)),
        ParseError::UnclosedSubgraph { opened_at, .. } => Some((*opened_at, 1)),
        ParseError::UnexpectedToken { line, col, .. }
        | ParseError::InvalidDirective { line, col, .. } => Some((*line, *col)),
        _ => None,
    };
    let Some((line, col)) = position else {
        return (0, diagram.len()).into();
    };
    let Some(text) = diagram.lines().nth(line.saturating_sub(1) as usize) else {
        return (0, diagram.len()).into();
    };
    // `lines()` borrows from `diagram`, so the line's own address gives its
    // offset without counting bytes a second time.
    let line_start = text.as_ptr() as usize - diagram.as_ptr() as usize;
    // The column counts characters, not bytes.
    let within = text
        .char_indices()
        .nth(col.saturating_sub(1) as usize)
        .map_or(text.len(), |(index, _)| index);
    (line_start + within, text.len() - within).into()
}

/// Splits a fence's parameters on commas, ignoring commas inside parentheses.
///
/// Without the exception `background=rgb(30,41,59)` would break into three
/// fragments and be rejected as "parameter `41` is not `key=value`" — an error
/// pointing nowhere near the actual mistake, for a value the docs say is
/// allowed.
/// An unbalanced `(` is refused rather than left to swallow the rest: with the
/// depth never returning to zero, `class=foo(,width=60%` parsed as one
/// parameter, and `width` vanished with no error at all.
fn split_params(params: &str) -> std::result::Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (index, chr) in params.char_indices() {
        match chr {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Err(format!("unmatched `)` in `{params}`"));
                }
                depth -= 1;
            }
            // A comma is one byte, so `index + 1` stays on a char boundary.
            ',' if depth == 0 => {
                parts.extend(params.get(start..index));
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(format!("unclosed `(` in `{params}`"));
    }
    parts.extend(params.get(start..));
    Ok(parts)
}

/// Whether a value names a colour SVG can paint with.
///
/// A shape check alone would not do the job: the typo worth catching is a
/// misspelled `transparent`, which is still a perfectly well-shaped word. So
/// bare names are matched against the actual CSS list, and only hex and the
/// functional notations are accepted structurally.
fn is_color(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8)
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if let Some((function, arguments)) = value.split_once('(') {
        let Some(arguments) = arguments.strip_suffix(')') else {
            return false;
        };
        return match function {
            "rgb" | "rgba" | "hsl" | "hsla" => has_components(arguments, 3..=4),
            _ => false,
        };
    }
    NAMED_COLORS.contains(&value)
}

/// Whether a functional colour's arguments are the right count of numbers.
///
/// Checking only that the notation *closes* let `rgb(banana)` through to
/// `fill="…"`, where an unpaintable value renders black — the silently wrong
/// slide `Background::parse` exists to prevent. The components are not range
/// checked: SVG clamps an out-of-range channel, so `rgb(300,0,0)` is merely
/// bright red, not a mistake worth failing a build over.
fn has_components(arguments: &str, count: std::ops::RangeInclusive<usize>) -> bool {
    // Both the legacy `r, g, b` and the modern `r g b / a` spellings are in use.
    let components = arguments
        .replace([',', '/'], " ")
        .split_whitespace()
        .map(|component| {
            let number = component.strip_suffix('%').unwrap_or(component);
            number.parse::<f32>().is_ok_and(f32::is_finite)
        })
        .collect::<Vec<_>>();
    count.contains(&components.len()) && components.iter().all(|valid| *valid)
}

/// The CSS named colours, minus `transparent`, which `Background` handles as a
/// mode of its own rather than as a paint.
const NAMED_COLORS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

/// Whether the config file moved a setting off the renderer's own baseline.
///
/// Neither `Theme` nor `LayoutConfig` derives `PartialEq`, but both derive
/// `Serialize`, so their serialized forms stand in for the comparison. This is
/// only ever run twice per build, on two small structs.
fn differs<T: serde::Serialize>(configured: &T, baseline: &T) -> bool {
    match (
        serde_json::to_value(configured),
        serde_json::to_value(baseline),
    ) {
        (Ok(configured), Ok(baseline)) => configured != baseline,
        // Neither type can fail to serialize. If one somehow did, treating it
        // as changed honours the file, which is what the author asked for.
        _ => true,
    }
}

/// The top-level keys the renderer's config file accepts.
///
/// Taken from `mermaid_rs_renderer 0.3.1`'s own `ConfigFile`, which is private,
/// so this list cannot be derived from it and has to be kept in step by hand.
const CONFIG_KEYS: &[&str] = &[
    "theme",
    "themeVariables",
    "preferredAspectRatio",
    "fastTextMetrics",
    "flowchart",
    "pie",
    "requirement",
    "mindmap",
    "gitGraph",
    "c4",
    "treemap",
    "timeline",
];

/// Rejects what the renderer's own loader would accept and quietly ignore, and
/// hands back the keys the file actually names.
///
/// `ConfigFile` has no `deny_unknown_fields`, and an unknown *theme name* is
/// swallowed by a `if let Some(theme) = Theme::from_name(…)` with no `else` —
/// which does not leave the deck alone, it silently swaps every diagram to
/// Mermaid's classic theme. A fence rejects both spellings of that mistake, so
/// the config file should not be the one place a typo is allowed to do nothing.
fn validate_config(path: &Path) -> Result<serde_json::Map<String, serde_json::Value>> {
    let reject = |message: String| TobogganCliError::MermaidConfig {
        path: path.to_path_buf(),
        message,
    };
    let text = std::fs::read_to_string(path).map_err(|err| reject(err.to_string()))?;
    let declared = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|err| reject(format!("not valid JSON: {err}")))?;
    let serde_json::Value::Object(keys) = declared else {
        return Err(reject("expected a JSON object".to_owned()));
    };
    for (key, value) in &keys {
        if !CONFIG_KEYS.contains(&key.as_str()) {
            return Err(reject(format!(
                "unknown setting `{key}` (expected one of {})",
                CONFIG_KEYS.join(", ")
            )));
        }
        // Every setting is optional, so leaving one out is how it is left
        // unset. An explicit `null` therefore says nothing the file could not
        // say by omission, and reads as a half-finished edit.
        if value.is_null() {
            return Err(reject(format!(
                "`{key}` is null; omit the key instead of setting it to null"
            )));
        }
    }
    match keys.get("theme") {
        None => Ok(keys),
        Some(serde_json::Value::String(name)) if Theme::from_name(name).is_some() => Ok(keys),
        Some(serde_json::Value::String(name)) => Err(reject(format!(
            "unknown theme `{name}` (expected {THEMES})"
        ))),
        Some(_) => Err(reject(format!("`theme` expects a name ({THEMES})"))),
    }
}

fn parse_number<T: std::str::FromStr>(key: &str, value: &str) -> std::result::Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("`{key}` expects a number, got `{value}`"))
}

/// A spacing or ratio, which the layout engine can only use if it is a real
/// positive number.
///
/// `"NaN"` and `"inf"` both parse as `f32`, and a `> 0.0` guard is *false* for
/// `NaN` rather than true — so without an explicit finiteness check they reach
/// the layout engine and produce a diagram with no coordinates.
fn parse_positive(key: &str, value: &str) -> std::result::Result<f32, String> {
    let number = parse_number::<f32>(key, value)?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("`{key}` expects a positive number, got `{value}`"));
    }
    Ok(number)
}

fn parse_bool(key: &str, value: &str) -> std::result::Result<bool, String> {
    match value {
        "true" | "yes" | "on" => Ok(true),
        "false" | "no" | "off" => Ok(false),
        other => Err(format!("`{key}` expects true or false, got `{other}`")),
    }
}

/// Accepts Mermaid's `16:9` spelling as well as a bare ratio.
fn parse_aspect_ratio(value: &str) -> std::result::Result<f32, String> {
    let ratio = match value.split_once(':') {
        Some((width, height)) => {
            let width = parse_positive("aspectRatio", width.trim())?;
            let height = parse_positive("aspectRatio", height.trim())?;
            width / height
        }
        None => parse_positive("aspectRatio", value)?,
    };
    // Both sides are finite and positive, but their quotient need not be: a
    // wide enough spread overflows to infinity.
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err(format!("`aspectRatio` must be positive, got `{value}`"));
    }
    Ok(ratio)
}

/// The byte offset of `part` inside `whole`, which it must be a subslice of.
///
/// Both come from the same buffer — `split_params` only ever slices — so the
/// address difference is the offset, with no second pass over the bytes.
fn offset_within(whole: &str, part: &str) -> usize {
    (part.as_ptr() as usize).saturating_sub(whole.as_ptr() as usize)
}

/// `spanned` is the sub-slice of `params` the caret should sit under.
fn fence_error(source_name: &str, params: &str, spanned: &str, reason: &str) -> TobogganCliError {
    let span = (offset_within(params, spanned), spanned.len()).into();
    TobogganCliError::invalid_mermaid_fence(source_name, params.to_owned(), span, reason.to_owned())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const FLOWCHART: &str = "flowchart LR\n  A[Start] --> B[End]\n";

    fn renderer() -> MermaidRenderer {
        MermaidRenderer::default()
    }

    #[test]
    fn a_bare_mermaid_fence_is_recognised() {
        assert!(renderer().parse_info("mermaid", "s.md").is_some());
    }

    #[test]
    fn a_non_mermaid_fence_is_left_alone() {
        for info in ["rust", "", "mermaidjs", "js:mermaid"] {
            assert!(
                renderer().parse_info(info, "s.md").is_none(),
                "`{info}` should not be treated as mermaid"
            );
        }
    }

    #[test]
    fn params_follow_a_colon_or_a_space() {
        for info in ["mermaid:theme=dark", "mermaid theme=dark"] {
            let fence = renderer()
                .parse_info(info, "s.md")
                .expect("mermaid fence")
                .expect("valid params");
            assert!(
                fence.options.theme.background != Theme::mermaid_default().background,
                "`{info}` did not apply the dark theme"
            );
        }
    }

    /// A colon inside a value survives, because only the *first* separator
    /// splits the language off.
    #[test]
    fn aspect_ratio_keeps_its_colon() {
        let fence = renderer()
            .parse_info("mermaid:aspectRatio=16:9,rankSpacing=80", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let ratio = fence
            .options
            .layout
            .preferred_aspect_ratio
            .expect("aspect ratio");
        assert!((ratio - 16.0 / 9.0).abs() < f32::EPSILON, "got {ratio}");
        assert!((fence.options.layout.rank_spacing - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn an_unknown_param_is_rejected_by_name() {
        let error = renderer()
            .parse_info("mermaid:nodeSpaceing=40", "deck/slide.md")
            .expect("mermaid fence")
            .expect_err("unknown parameter");
        let rendered = error.to_string();
        assert!(rendered.contains("nodeSpaceing"), "{rendered}");
        assert!(rendered.contains("deck/slide.md"), "{rendered}");
    }

    #[test]
    fn a_param_without_a_value_is_rejected() {
        renderer()
            .parse_info("mermaid:theme", "s.md")
            .expect("mermaid fence")
            .expect_err("`theme` alone is not key=value");
    }

    #[test]
    fn an_unknown_theme_is_rejected() {
        renderer()
            .parse_info("mermaid:theme=solarized", "s.md")
            .expect("mermaid fence")
            .expect_err("unknown theme");
    }

    #[test]
    fn a_broken_diagram_fails_the_build_and_names_the_file() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "deck/broken.md")
            .expect("mermaid fence")
            .expect("valid params");
        let error = renderer
            .render_svg(&fence, "not a diagram at all", "deck/broken.md")
            .expect_err("diagram should not parse");
        assert!(error.to_string().contains("deck/broken.md"), "{error}");
    }

    #[test]
    fn rendering_produces_a_self_contained_svg() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let svg = renderer
            .render_svg(&fence, FLOWCHART, "s.md")
            .expect("render");
        assert!(svg.starts_with("<svg "), "{svg}");
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(!svg.contains("src=\"http"), "no external references");
        assert!(!svg.contains("url(http"), "no external references");
    }

    /// The whole point of namespacing: two diagrams in one document must not
    /// resolve each other's markers.
    #[test]
    fn two_diagrams_do_not_share_ids() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let first = renderer
            .render_svg(&fence, "flowchart LR\n  A --> B\n", "s.md")
            .expect("render");
        let second = renderer
            .render_svg(&fence, "flowchart TD\n  X --> Y\n  Y --> Z\n", "s.md")
            .expect("render");

        let first_ids = collect_ids(&first);
        let second_ids = collect_ids(&second);
        assert!(!first_ids.is_empty(), "expected ids to namespace");
        for id in &first_ids {
            assert!(
                !second_ids.contains(id),
                "`{id}` appears in both diagrams: {first_ids:?} / {second_ids:?}"
            );
        }
    }

    /// The diagram kinds that reference their markers through `url(#…)`.
    ///
    /// A flowchart draws its arrowheads as literal `<polygon>`s and emits no
    /// `url(#…)` at all, so a test written against one asserts nothing about
    /// reference rewriting — the loop body simply never runs. These three do
    /// emit them, and are what would silently lose their arrowheads if the
    /// `url(#` arm of `rewrite_reference` were dropped.
    const REFERENCING: [(&str, &str); 3] = [
        (
            "sequence",
            "sequenceDiagram\n  Alice->>Bob: Hi\n  Bob-->>Alice: Hello\n",
        ),
        ("class", "classDiagram\n  Animal <|-- Duck\n"),
        (
            "state",
            "stateDiagram-v2\n  [*] --> Still\n  Still --> Moving\n",
        ),
    ];

    fn references(svg: &str) -> Vec<String> {
        svg.split("url(#")
            .skip(1)
            .map(|reference| {
                reference
                    .split_once(')')
                    .map(|(name, _)| name.to_owned())
                    .expect("closing paren")
            })
            .collect()
    }

    /// Every `url(#…)` must still point at something that exists.
    #[test]
    fn namespacing_leaves_no_dangling_reference() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        for (kind, diagram) in REFERENCING {
            let svg = renderer
                .render_svg(&fence, diagram, "s.md")
                .expect("render");
            let found = references(&svg);
            // Guard against this test quietly going vacuous again if upstream
            // changes how a diagram draws its markers.
            assert!(!found.is_empty(), "{kind}: no `url(#…)` to check in {svg}");
            let ids = collect_ids(&svg);
            for name in found {
                assert!(ids.contains(&name), "{kind}: dangling `{name}` in {svg}");
            }
        }
    }

    /// The shape that actually ships: the HTML export puts every slide in one
    /// document, so ids must be unique and references resolve *across* the
    /// diagrams, not merely within each one.
    #[test]
    fn diagrams_sharing_a_document_keep_their_own_markers() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let document = REFERENCING
            .iter()
            .map(|(_, diagram)| {
                renderer
                    .render_svg(&fence, diagram, "s.md")
                    .expect("render")
            })
            .collect::<Vec<_>>()
            .concat();

        let ids = collect_ids(&document);
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            ids.len(),
            unique.len(),
            "duplicate id across diagrams: {ids:?}"
        );

        for name in references(&document) {
            assert!(ids.contains(&name), "dangling `{name}` in the document");
        }
    }

    /// `data-edge-id="edge-0"` is a label, not an id, and must not be rewritten.
    #[test]
    fn data_attributes_ending_in_id_are_not_treated_as_ids() {
        let svg = r#"<svg><path id="edge-0" data-edge-id="edge-0"/></svg>"#;
        assert_eq!(collect_ids(svg), vec!["edge-0".to_owned()]);
        let namespaced = namespace_ids(svg);
        assert!(
            namespaced.contains(r#"data-edge-id="edge-0""#),
            "{namespaced}"
        );
        assert!(!namespaced.contains(r#" id="edge-0""#), "{namespaced}");
    }

    #[test]
    fn the_background_rectangle_is_dropped_by_default() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let svg = renderer
            .render_svg(&fence, FLOWCHART, "s.md")
            .expect("render");
        assert!(
            !svg.contains("#FFFFFF\"/><"),
            "background rect survived: {svg}"
        );
    }

    /// A pasted fence often carries a non-breaking space. `char::is_whitespace`
    /// is true for it, but it is two bytes wide — and slicing mid-character used
    /// to leave `parse_info` returning `None`, which every caller reads as "not
    /// a Mermaid fence", so the diagram silently became a code block.
    #[test]
    fn a_multi_byte_separator_still_marks_a_mermaid_fence() {
        for info in ["mermaid\u{a0}theme=dark", "mermaid\u{2028}theme=dark"] {
            let fence = renderer()
                .parse_info(info, "s.md")
                .unwrap_or_else(|| panic!("{info:?} was not recognised as a Mermaid fence"))
                .expect("valid params");
            drop(fence);
        }
    }

    /// The caret must land on the line the renderer complained about, so an
    /// author with a twenty-line diagram is not left to find it themselves.
    #[test]
    fn a_broken_diagram_is_spanned_at_the_offending_line() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "deck/broken.md")
            .expect("mermaid fence")
            .expect("valid params");
        let diagram = "flowchart LR\n  A --> B\n  --> nonsense\n";
        let error = renderer
            .render_svg(&fence, diagram, "deck/broken.md")
            .expect_err("diagram should not parse");

        let TobogganCliError::InvalidMermaid { src, span, .. } = &error else {
            panic!("unexpected variant: {error:?}");
        };
        assert!(
            src.name().starts_with("deck/broken.md"),
            "the slide is not named: {}",
            src.name()
        );
        let spanned = src
            .inner()
            .get(span.offset()..span.offset() + span.len())
            .expect("span inside the diagram");
        assert!(
            spanned.contains("nonsense"),
            "caret landed on {spanned:?}, not the broken line"
        );
    }

    /// A variant the renderer gives no position for must still produce a span
    /// inside the diagram rather than an empty or out-of-range one.
    #[test]
    fn a_span_always_stays_inside_the_diagram() {
        let diagram = "flowchart LR\n  A --> B\n";
        for line in [0, 1, 2, 99] {
            let error = ParseError::UnexpectedToken {
                line,
                col: 200,
                found: "x".to_owned(),
                expected: "y".to_owned(),
            };
            let span = error_span(&error, diagram);
            assert!(
                span.offset() + span.len() <= diagram.len(),
                "line {line}: {span:?} runs past {} bytes",
                diagram.len()
            );
        }
    }

    #[test]
    fn a_spacing_must_be_a_real_positive_number() {
        for params in [
            "nodeSpacing=NaN",
            "nodeSpacing=inf",
            "nodeSpacing=-40",
            "nodeSpacing=0",
            "rankSpacing=NaN",
            "aspectRatio=NaN",
            "aspectRatio=inf",
            "aspectRatio=16:0",
            "aspectRatio=0:5",
            "aspectRatio=NaN:1",
        ] {
            params_of(params).expect_err(params);
        }
    }

    /// A rejected parameter must not be reported as a bad *diagram*: the advice
    /// on that variant is to fix the diagram or fence it as text, which for a
    /// misspelled parameter points at a diagram that is fine.
    #[test]
    fn a_rejected_parameter_is_a_fence_error_not_a_diagram_error() {
        let error = renderer()
            .parse_info("mermaid:theme=drak", "s.md")
            .expect("mermaid fence")
            .expect_err("unknown theme");
        assert!(
            matches!(&error, TobogganCliError::InvalidMermaidFence { src, .. } if src.name().starts_with("s.md")),
            "unexpected variant: {error:?}"
        );
    }

    fn params_of(params: &str) -> std::result::Result<MermaidFence, String> {
        renderer()
            .parse_info(&format!("mermaid:{params}"), "s.md")
            .expect("mermaid fence")
            .map_err(|error| error.to_string())
    }

    /// The typo worth catching is a well-shaped word, so before this it went
    /// straight into `fill="…"` as an invalid paint, which renders black — a
    /// silently wrong slide, where a misspelled `theme` is a build error.
    #[test]
    fn a_misspelled_background_is_refused_rather_than_painted() {
        for params in [
            "background=darkslateblu",
            "background=thmee",
            "background=#12345",
        ] {
            let error = params_of(params).expect_err(params);
            assert!(error.contains("s.md"), "{params}: {error}");
            assert!(error.contains("`background`"), "{params}: {error}");
        }
    }

    #[test]
    fn a_background_takes_a_mode_or_a_real_colour() {
        for params in [
            "background=transparent",
            "background=none",
            "background=theme",
            "background=default",
            "background=white",
            "background=rebeccapurple",
            "background=#fff",
            "background=#1E293B",
            "background=#1E293BFF",
        ] {
            params_of(params).unwrap_or_else(|error| panic!("{params}: {error}"));
        }
    }

    /// A functional colour carries commas, which used to split it into three
    /// fragments and fail with an error naming `41`.
    #[test]
    fn a_colour_may_contain_commas() {
        let fence = params_of("background=rgb(30,41,59),width=40%").expect("valid params");
        assert_eq!(
            fence.width().map(|width| width.to_string()),
            Some("40%".to_owned()),
            "the parameter after the colour was lost"
        );
    }

    /// A stray `(` used to make `depth` never return to zero, so every later
    /// parameter was absorbed into this one and silently did nothing — the
    /// width below simply vanished, with no error and a successful build.
    #[test]
    fn an_unbalanced_parenthesis_is_refused_rather_than_swallowing_the_rest() {
        for params in [
            "class=foo(,width=60%",
            "alt=Step 3 (the fan-out",
            "background=rgb(30,41,59)),width=60%",
        ] {
            let error = params_of(params).expect_err(params);
            assert!(
                error.contains("unclosed `(`") || error.contains("unmatched `)`"),
                "{params}: {error}"
            );
        }
    }

    /// The check exists to stop an unpaintable value reaching `fill="…"`, where
    /// it renders black. Closing the parenthesis was the only thing asked of it,
    /// so `rgb(banana)` passed and painted a black box.
    #[test]
    fn a_functional_colour_must_hold_real_components() {
        for params in [
            "background=rgb(banana)",
            "background=rgb(30,41)",
            "background=rgb(30,41,59,0.5,7)",
            "background=hsl(nonsense,10%,20%)",
            "background=rgb(30,41,59",
        ] {
            params_of(params).expect_err(params);
        }
        for params in [
            "background=rgb(30,41,59)",
            "background=rgba(30,41,59,0.5)",
            "background=hsl(210,40%,20%)",
            "background=rgb(30 41 59)",
            "background=rgb(30 41 59 / 0.5)",
        ] {
            params_of(params).unwrap_or_else(|error| panic!("{params}: {error}"));
        }
    }

    /// An accessible label is a sentence, and a sentence has commas in it. They
    /// used to split it into fragments, so the label was rejected for naming a
    /// parameter the author never wrote.
    #[test]
    fn an_alt_label_may_contain_commas() {
        let fence = params_of("width=60%,alt=Write, then build, then present")
            .expect("a label with commas");
        assert_eq!(fence.alt(), Some("Write, then build, then present"));
        assert_eq!(
            fence.width().map(|width| width.to_string()),
            Some("60%".to_owned()),
            "a parameter before the label was lost"
        );

        let only = params_of("alt=A, B, C").expect("a label alone");
        assert_eq!(only.alt(), Some("A, B, C"));
    }

    #[test]
    fn the_named_colours_hold_no_duplicates() {
        let mut sorted = NAMED_COLORS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), NAMED_COLORS.len());
    }

    fn width_of(params: &str) -> std::result::Result<Option<Length>, String> {
        renderer()
            .parse_info(&format!("mermaid:{params}"), "s.md")
            .expect("mermaid fence")
            .map(|fence| fence.width())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn a_width_keeps_its_number_and_unit() {
        let width = width_of("width=38%").expect("valid").expect("a width");
        assert_eq!(width.to_string(), "38%");
        let width = width_of("width=8.5cm").expect("valid").expect("a width");
        assert_eq!(width.to_string(), "8.5cm");
    }

    /// The Typst-injection barrier rests on `f32`'s `Display` never reaching for
    /// exponent notation — `width: 3.8e1%` is not a Typst length. That is a
    /// property of the standard library rather than of this code, and the other
    /// width tests all use small round numbers, so nothing else would notice a
    /// `{:e}` creeping into `Display`.
    #[test]
    fn a_width_never_renders_in_exponent_notation() {
        for params in [
            "width=0.0000001cm",
            "width=340000000000000000000000000000000000000pt",
            "width=0.00000000000000000000000000000000000001%",
        ] {
            let width = width_of(params)
                .unwrap_or_else(|error| panic!("{params}: {error}"))
                .expect("a width");
            let rendered = width.to_string();
            assert!(
                !rendered.contains('e') && !rendered.contains('E'),
                "{params} rendered as {rendered}, which Typst cannot parse"
            );
        }
    }

    /// `px` is the unit an author reaches for first, and Typst has no such
    /// length — so accepting it would build a deck that renders on the
    /// projector and fails `toboggan pdf`. Reject it where every other bad
    /// parameter is rejected: at build time, naming the file.
    #[test]
    fn a_width_in_a_unit_only_css_understands_is_refused() {
        for params in ["width=200px", "width=10rem", "width=50vw"] {
            let error = width_of(params).expect_err(params);
            assert!(error.contains("s.md"), "{params}: {error}");
            assert!(error.contains("CSS and Typst"), "{params}: {error}");
        }
    }

    /// The width is interpolated into generated Typst source next to the
    /// escaped SVG. Anything that is not a number and a known unit could close
    /// the `image(…)` call it sits in and have the rest compiled as code.
    #[test]
    fn a_width_cannot_carry_typst_source() {
        let error = width_of(r#"width=1cm)) #text[#read("secret.txt")] #box(box("#)
            .expect_err("injection accepted");
        assert!(error.contains("s.md"), "{error}");
    }

    #[test]
    fn a_width_must_be_a_positive_length() {
        for params in ["width=%", "width=0%", "width=-4cm", "width=60", "width="] {
            width_of(params).expect_err(params);
        }
    }

    fn config_renderer(json: &str) -> MermaidRenderer {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("mermaid.json");
        std::fs::write(&path, json).expect("write config");
        MermaidRenderer::from_config(Some(&path)).expect("load config")
    }

    fn render_with(renderer: &MermaidRenderer) -> String {
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        renderer
            .render_svg(&fence, FLOWCHART, "s.md")
            .expect("render")
    }

    /// A config file that only retints diagrams must not also change how their
    /// labels are measured: the crate's own layout defaults measure against the
    /// building machine's fonts, which would make the committed `.toml`
    /// artifacts differ per machine. See `default_options`.
    #[test]
    fn a_config_file_does_not_silently_change_the_defaults_it_omits() {
        assert_eq!(
            render_with(&config_renderer("{}")),
            render_with(&MermaidRenderer::default()),
            "an empty config file changed how a diagram is drawn"
        );
    }

    #[test]
    fn a_config_file_can_still_ask_for_font_measured_labels() {
        let configured = config_renderer(r#"{"fastTextMetrics": false}"#);
        assert!(
            !configured.base.layout.fast_text_metrics,
            "an explicit opt-out was ignored"
        );
    }

    /// Asserting merely that the render *differs* from the default would also
    /// pass for a theme we never asked for — which is exactly how a misspelled
    /// theme name went unnoticed. Pin the theme that was actually requested.
    #[test]
    fn a_config_file_still_applies_the_keys_it_sets() {
        let configured = config_renderer(r#"{"theme": "dark"}"#);
        let mut wanted = default_options();
        wanted.theme = Theme::dark();
        assert_eq!(
            render_with(&configured),
            render_with(&MermaidRenderer { base: wanted }),
            "the config file's theme was not applied"
        );
    }

    /// `themeVariables` is folded into `config.theme` rather than kept under a
    /// key of its own, so gating on the presence of a top-level `theme` key
    /// discarded every variable a retinting config set — silently, which is the
    /// one outcome this whole feature is written against.
    #[test]
    fn a_config_file_of_theme_variables_alone_is_applied() {
        let configured = config_renderer(r##"{"themeVariables": {"primaryColor": "#ff0000"}}"##);
        let svg = render_with(&configured);
        assert!(
            svg.contains("#ff0000") || svg.contains("#FF0000"),
            "the config file's themeVariables were dropped: {svg}"
        );
    }

    /// The renderer's own loader swallows an unknown theme name, which does not
    /// leave the deck alone — it silently swaps every diagram to Mermaid's
    /// classic theme. A fence rejects the same typo, so the config file must too.
    #[test]
    fn a_config_file_typo_is_refused_rather_than_ignored() {
        for (json, expected) in [
            (r#"{"theme": "solarized"}"#, "unknown theme"),
            (r#"{"theme": 3}"#, "expects a name"),
            (r#"{"theme": null}"#, "omit the key"),
            (r#"{"themeVaraibles": {}}"#, "unknown setting"),
            (r#"{"fastTextMetrics": null}"#, "omit the key"),
            (r#"{"theme": "dark", "bogus": 1}"#, "unknown setting"),
            ("[]", "expected a JSON object"),
        ] {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().join("mermaid.json");
            std::fs::write(&path, json).expect("write config");
            let error =
                MermaidRenderer::from_config(Some(&path)).expect_err(&format!("{json} accepted"));
            let rendered = error.to_string();
            assert!(rendered.contains(expected), "{json}: {rendered}");
        }
    }

    /// Both surfaces defer to `Theme::from_name`, so both must name the same
    /// set — otherwise a name works in one place and is a typo in the other.
    #[test]
    fn a_fence_and_a_config_accept_the_same_theme_names() {
        for name in [
            "default", "base", "mermaid", "dark", "forest", "neutral", "modern",
        ] {
            params_of(&format!("theme={name}"))
                .unwrap_or_else(|error| panic!("fence rejected `{name}`: {error}"));
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().join("mermaid.json");
            std::fs::write(&path, format!(r#"{{"theme": "{name}"}}"#)).expect("write config");
            MermaidRenderer::from_config(Some(&path))
                .unwrap_or_else(|error| panic!("config rejected `{name}`: {error}"));
        }
    }

    #[test]
    fn a_missing_config_file_is_an_error_naming_the_path() {
        let error = MermaidRenderer::from_config(Some(Path::new("no/such/mermaid.json")))
            .expect_err("missing config accepted");
        assert!(
            matches!(&error, TobogganCliError::MermaidConfig { path, .. }
                if path.ends_with("mermaid.json")),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn the_background_can_be_kept_or_repainted() {
        let renderer = renderer();
        let kept = renderer
            .parse_info("mermaid:background=theme", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let svg = renderer
            .render_svg(&kept, FLOWCHART, "s.md")
            .expect("render");
        assert!(svg.contains("#FFFFFF"), "{svg}");

        let painted = renderer
            .parse_info("mermaid:background=#123456", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let svg = renderer
            .render_svg(&painted, FLOWCHART, "s.md")
            .expect("render");
        assert!(svg.contains(r##"fill="#123456""##), "{svg}");
    }

    /// Only the background rectangle is eligible; a real shape stays put.
    #[test]
    fn a_rect_that_is_not_the_background_is_kept() {
        let svg = r##"<svg viewBox="0 0 2 2"><rect x="0" y="0" width="2" height="2" rx="3" fill="#fff"/></svg>"##;
        assert!(find_background_rect(svg).is_none());
    }

    #[test]
    fn html_wraps_the_diagram_and_carries_the_alt_text() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid:class=wide,width=70%,alt=A flow", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let html = renderer
            .render_html(&fence, FLOWCHART, "s.md")
            .expect("render");
        assert!(
            html.starts_with(r#"<div class="mermaid mermaid-sized wide" style="width: 70%""#),
            "{html}"
        );
        assert!(html.contains(r#"role="img" aria-label="A flow""#), "{html}");
        assert!(html.contains("<svg "), "{html}");
        assert!(html.ends_with("</div>\n"), "{html}");
    }
}
