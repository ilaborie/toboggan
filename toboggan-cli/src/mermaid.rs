//! Renders ` ```mermaid ` fences to SVG while the deck builds.
//!
//! This mirrors what the parser already does for `$…$` math: the conversion
//! happens here rather than in the browser, so an exported deck draws its
//! diagrams with no script, no CDN and no web font, and a broken diagram stops
//! the build and names the file instead of rendering as nothing in front of an
//! audience.
//!
//! Two independent pipelines consume the same fence — [`crate::parser`] for the
//! HTML the web client and the HTML export read, and the Typst renderer behind
//! [`crate::output`] for the PDF and the thumbnails — so both go through
//! [`MermaidRenderer`] and cannot disagree about what a fence means.

use std::fmt::Write as _;
use std::path::Path;

use mermaid_rs_renderer::{RenderOptions, Theme, render_strict};

use crate::error::{Result, TobogganCliError};
use crate::output::html::escape_html;

/// The fence language that marks a Mermaid diagram.
const FENCE_LANG: &str = "mermaid";

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
/// between machines. The fast path uses calibrated widths and never opens the
/// font database. It is also the fastest option.
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
        let config = mermaid_rs_renderer::config::load_config(Some(path)).map_err(|err| {
            TobogganCliError::MermaidConfig {
                path: path.to_path_buf(),
                message: err.to_string(),
            }
        })?;
        let mut base = default_options();
        base.theme = config.theme;
        base.layout = config.layout;
        Ok(Self { base })
    }

    /// Parses a fence info string.
    ///
    /// Returns `None` when the fence is not a Mermaid one, so a caller can fall
    /// through to its normal code-block handling.
    ///
    /// # Errors
    /// Returns [`TobogganCliError::InvalidMermaid`] for a malformed or unknown
    /// parameter, so a typo is reported rather than silently ignored.
    #[must_use]
    pub fn parse_info(&self, info: &str, source_name: &str) -> Option<Result<MermaidFence>> {
        let info = info.trim();
        let separator = info.find(|chr: char| chr == ':' || chr.is_whitespace());
        let (lang, params) = match separator {
            Some(index) => (info.get(..index)?, info.get(index + 1..)?),
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
        for param in params.split(',').map(str::trim).filter(|it| !it.is_empty()) {
            let (key, value) = param.split_once('=').ok_or_else(|| {
                fence_error(
                    source_name,
                    params,
                    &format!("parameter `{param}` is not `key=value`"),
                )
            })?;
            fence
                .apply(key.trim(), value.trim())
                .map_err(|reason| fence_error(source_name, params, &reason))?;
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
            TobogganCliError::InvalidMermaid {
                file: source_name.to_owned(),
                diagram: diagram.to_owned(),
                message: err.to_string(),
            }
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
        if let Some(width) = &fence.width {
            let _ = write!(open, r#" style="width: {}""#, escape_html(width));
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
    /// CSS / Typst length for the rendered diagram.
    width: Option<String>,
    /// Extra classes for the HTML wrapper.
    class: Option<String>,
    /// Accessible label, and the text stand-in outside the web.
    alt: Option<String>,
}

impl MermaidFence {
    /// The explicit size the author asked for, as a CSS or Typst length.
    #[must_use]
    pub fn width(&self) -> Option<&str> {
        self.width.as_deref()
    }

    /// Applies one `key=value` parameter, reporting the reason it was rejected.
    fn apply(&mut self, key: &str, value: &str) -> std::result::Result<(), String> {
        match key {
            "theme" => {
                self.options.theme = Theme::from_name(value).ok_or_else(|| {
                    format!(
                        "unknown theme `{value}` (expected default, dark, forest, neutral or modern)"
                    )
                })?;
            }
            "background" => self.background = Background::parse(value),
            "nodeSpacing" => self.options.layout.node_spacing = parse_number(key, value)?,
            "rankSpacing" => self.options.layout.rank_spacing = parse_number(key, value)?,
            "maxLabelWidth" => {
                self.options.layout.max_label_width_chars = parse_number(key, value)?;
            }
            "aspectRatio" => {
                self.options.layout.preferred_aspect_ratio = Some(parse_aspect_ratio(value)?);
            }
            "fastText" => self.options.layout.fast_text_metrics = parse_bool(key, value)?,
            "width" => self.width = Some(value.to_owned()),
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

/// What sits behind the diagram.
///
/// Mermaid paints an opaque page-coloured rectangle over the whole viewBox,
/// which on a themed slide reads as a white box punched into the design. A
/// slide almost always wants the deck's own background showing through, so that
/// rectangle is dropped unless the author asks for it back.
#[derive(Debug, Clone)]
enum Background {
    /// Drop Mermaid's background rectangle.
    Transparent,
    /// Keep whatever the theme paints.
    Theme,
    /// Repaint the rectangle in this colour.
    Color(String),
}

impl Background {
    fn parse(value: &str) -> Self {
        match value {
            "transparent" | "none" => Self::Transparent,
            "theme" | "default" => Self::Theme,
            color => Self::Color(color.to_owned()),
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

fn parse_number<T: std::str::FromStr>(key: &str, value: &str) -> std::result::Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("`{key}` expects a number, got `{value}`"))
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
            let width = parse_number::<f32>("aspectRatio", width.trim())?;
            let height = parse_number::<f32>("aspectRatio", height.trim())?;
            if height == 0.0 {
                return Err("`aspectRatio` height must not be zero".to_owned());
            }
            width / height
        }
        None => parse_number::<f32>("aspectRatio", value)?,
    };
    if ratio <= 0.0 {
        return Err(format!("`aspectRatio` must be positive, got `{value}`"));
    }
    Ok(ratio)
}

fn fence_error(source_name: &str, params: &str, reason: &str) -> TobogganCliError {
    TobogganCliError::InvalidMermaid {
        file: source_name.to_owned(),
        diagram: params.to_owned(),
        message: format!("`{FENCE_LANG}:{params}` — {reason}"),
    }
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

    /// Every `url(#…)` must still point at something that exists.
    #[test]
    fn namespacing_leaves_no_dangling_reference() {
        let renderer = renderer();
        let fence = renderer
            .parse_info("mermaid", "s.md")
            .expect("mermaid fence")
            .expect("valid params");
        let svg = renderer
            .render_svg(&fence, "flowchart LR\n  A --> B\n", "s.md")
            .expect("render");
        let ids = collect_ids(&svg);
        for reference in svg.split("url(#").skip(1) {
            let name = reference
                .split_once(')')
                .map(|(name, _)| name)
                .expect("closing paren");
            assert!(ids.contains(&name.to_owned()), "dangling `{name}` in {svg}");
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
