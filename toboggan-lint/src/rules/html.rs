use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// Nested `.step` elements break the frontend reveal logic.
pub(crate) struct NestedStep;

impl Rule for NestedStep {
    fn id(&self) -> RuleId {
        super::ids::HTML_NESTED_STEP
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let nested = context.body_doc().count_nested_steps();
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
        super::ids::HTML_IMG_MISSING_ALT
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let missing = context.body_doc().count_images_without_alt();
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
        super::ids::HTML_RAW_SCRIPT
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.body_doc().has_script() {
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
        super::ids::HTML_HEADING_H1
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let slide = context.slide;
        let title_rendered = !matches!(slide.title, toboggan_core::Content::Empty)
            && !slide.style.classes.iter().any(|class| class == "no_title");
        if title_rendered && context.body_doc().has_h1() {
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

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, Slide};

    use super::*;
    use crate::rules::test_support::{fires, only, slide_diagnostics};

    fn slide_with(body: &str) -> Slide {
        Slide::new("T").with_body(Content::html(body))
    }

    /// Nested steps break the frontend reveal logic outright, which is why
    /// this is the one rule at error severity.
    #[test]
    fn a_nested_step_is_an_error() {
        let slide = slide_with(r#"<div class="step">a<div class="step">b</div></div>"#);
        let diagnostics = slide_diagnostics(&NestedStep, &slide);
        assert_eq!(only(&diagnostics).severity, Severity::Error);
    }

    #[test]
    fn sibling_steps_are_not_nested() {
        let slide = slide_with(r#"<div class="step">a</div><div class="step">b</div>"#);
        assert!(!fires(&NestedStep, &slide));
    }

    /// An empty `alt` is as useless to a screen reader as a missing one, and
    /// is what an author writes when the markdown is `![](x.png)`.
    #[test]
    fn a_missing_or_empty_alt_is_reported() {
        for img in [r#"<img src="a.png">"#, r#"<img src="a.png" alt="">"#] {
            assert!(fires(&ImgMissingAlt, &slide_with(img)), "{img}");
        }
        assert!(!fires(
            &ImgMissingAlt,
            &slide_with(r#"<img src="a.png" alt="a chart">"#)
        ));
    }

    /// The check asks the parsed tree, not the raw text: `<script` inside a
    /// comment or a code sample is not a script.
    #[test]
    fn only_a_real_script_element_counts() {
        assert!(fires(&RawScript, &slide_with("<script>alert(1)</script>")));
        assert!(!fires(
            &RawScript,
            &slide_with("<p>write <code>&lt;script&gt;</code> to embed one</p>")
        ));
    }

    /// A body `<h1>` beside a rendered title gives the slide two top headings.
    #[test]
    fn an_h1_beside_a_rendered_title_is_reported() {
        assert!(fires(&HeadingH1, &slide_with("<h1>Also me</h1>")));
    }

    /// Two ways a body `<h1>` is legitimate: nothing else renders a title.
    /// The `no_title` exemption is the subtle one — the title exists but is
    /// hidden, so the body heading is the only one on screen.
    #[test]
    fn an_h1_is_fine_when_it_is_the_only_heading() {
        let untitled = Slide::default().with_body(Content::html("<h1>Only me</h1>"));
        assert!(!fires(&HeadingH1, &untitled));

        let hidden_title = Slide::new("T")
            .with_body(Content::html("<h1>Only me</h1>"))
            .with_style_classes(["no_title".to_owned()]);
        assert!(!fires(&HeadingH1, &hidden_title));
    }

    #[test]
    fn a_lesser_heading_is_always_fine() {
        assert!(!fires(&HeadingH1, &slide_with("<h2>Section</h2>")));
    }
}
