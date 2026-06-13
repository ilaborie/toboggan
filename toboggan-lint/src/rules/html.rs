use toboggan_stats::HtmlDocument;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext, body_html};

/// Nested `.step` elements break the frontend reveal logic.
pub(crate) struct NestedStep;

impl Rule for NestedStep {
    fn id(&self) -> RuleId {
        RuleId("html/nested-step")
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let nested = HtmlDocument::parse_fragment(body_html(context.slide)).count_nested_steps();
        if nested > 0 {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{nested} nested reveal step(s)"),
                )
                .with_help("steps cannot be nested; flatten the `pause` structure"),
            );
        }
    }
}

/// Images without alt text hurt accessibility and the PDF export.
pub(crate) struct ImgMissingAlt;

impl Rule for ImgMissingAlt {
    fn id(&self) -> RuleId {
        RuleId("html/img-missing-alt")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let missing =
            HtmlDocument::parse_fragment(body_html(context.slide)).count_images_without_alt();
        if missing > 0 {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{missing} image(s) without alt text"),
                )
                .with_help("add `![alt text](...)` so images have a description"),
            );
        }
    }
}

/// Raw `<script>` in slide content is an authoring/security smell.
pub(crate) struct RawScript;

impl Rule for RawScript {
    fn id(&self) -> RuleId {
        RuleId("html/raw-script")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if body_html(context.slide)
            .to_ascii_lowercase()
            .contains("<script")
        {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "slide body contains a raw <script> tag",
                )
                .with_help("avoid inline scripts in slides; use the talk _head.html if needed"),
            );
        }
    }
}

/// A slide body uses `<h1>` while ALSO rendering its title, producing two top
/// headings. Slides that hide the title (the `no_title` class) legitimately use
/// a body `# heading`, so those are not flagged.
pub(crate) struct HeadingH1;

impl Rule for HeadingH1 {
    fn id(&self) -> RuleId {
        RuleId("html/heading-h1")
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let slide = context.slide;
        let title_rendered = !matches!(slide.title, toboggan_core::Content::Empty)
            && !slide.style.classes.iter().any(|class| class == "no_title");
        if title_rendered && body_html(slide).to_ascii_lowercase().contains("<h1") {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "slide renders a title and also has an <h1> in the body",
                )
                .with_help(
                    "use `##`/`###` in the body, or hide the title with the `no_title` class",
                ),
            );
        }
    }
}
