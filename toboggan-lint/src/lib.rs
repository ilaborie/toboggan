//! Lints for Toboggan presentations.
//!
//! The library runs a set of [`Rule`]s over a parsed [`Talk`] and produces a
//! framework-neutral [`LintReport`]. It deliberately has no CLI/terminal
//! dependencies so it can be reused by `toboggan-cli` (rendered with miette) and
//! `toboggan-mcp` (serialized to JSON).

mod diagnostic;
mod report;
mod rule;
mod rules;

use toboggan_core::Talk;

use self::diagnostic::SlideRef as Ref;
pub use self::diagnostic::{LintDiagnostic, RuleId, Severity, SlideRef};
pub use self::report::LintReport;
pub use self::rule::{LintConfig, Rule, RuleContext};
pub use self::rules::all_rules;

/// Lints `talk` with the given configuration and returns a [`LintReport`].
#[must_use]
pub fn lint(talk: &Talk, config: &LintConfig) -> LintReport {
    let rules = all_rules();
    let mut out = Vec::new();

    // Talk-level rules first.
    for rule in &rules {
        if config.is_enabled(rule.id()) {
            rule.check_talk(talk, config, &mut out);
        }
    }

    // Per-slide rules.
    for (index, slide) in talk.slides.iter().enumerate() {
        let slide_ref = Ref::new(index, slide);
        let context = RuleContext {
            talk,
            slide,
            slide_ref: &slide_ref,
            config,
        };
        for rule in &rules {
            // A rule runs unless it is globally disabled or silenced for this
            // slide via front matter / a `<!-- lint-disable -->` body comment.
            // Talk-level rules are not affected by per-slide directives.
            let id = rule.id();
            let disabled_here = slide
                .lint_disabled
                .iter()
                .any(|disabled| disabled == id.as_str());
            if config.is_enabled(id) && !disabled_here {
                rule.check_slide(&context, &mut out);
            }
        }
    }

    // Apply per-rule severity overrides centrally.
    for diagnostic in &mut out {
        let default = diagnostic.severity;
        diagnostic.severity = config.severity(diagnostic.rule, default);
    }

    LintReport::new(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::{Content, Slide, SlideKind, Talk};

    use super::*;

    fn talk_with(slides: Vec<Slide>) -> Talk {
        let mut talk = Talk::new("Test");
        talk.slides = slides;
        talk
    }

    #[test]
    fn clean_talk_has_no_diagnostics() {
        let slide = Slide::new("Title").with_body(Content::html("<p>Hello world</p>"));
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        assert!(report.is_clean(), "unexpected: {:?}", report.diagnostics);
    }

    #[test]
    fn detects_pause_in_part() {
        let part = Slide {
            kind: SlideKind::Part,
            title: Content::text("Section"),
            body: Content::html(r#"<div class="step">oops</div>"#),
            ..Default::default()
        };
        let report = lint(&talk_with(vec![part]), &LintConfig::default());
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule.as_str() == "pause/in-part")
            .unwrap_or_else(|| panic!("expected pause/in-part, got {:?}", report.diagnostics));
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.errors, 0);
    }

    #[test]
    fn detects_pause_in_cover() {
        let cover = Slide {
            kind: SlideKind::Cover,
            title: Content::text("Welcome"),
            body: Content::html(r#"<div class="step">oops</div>"#),
            ..Default::default()
        };
        let report = lint(&talk_with(vec![cover]), &LintConfig::default());
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule.as_str() == "pause/in-part")
            .unwrap_or_else(|| panic!("expected pause/in-part, got {:?}", report.diagnostics));
        assert_eq!(diagnostic.severity, Severity::Warning);
    }

    #[test]
    fn per_slide_disable_suppresses_rule() {
        let slide = Slide::new("T")
            .with_body(Content::html(r#"<img src="a.png">"#))
            .with_lint_disabled(["html/img-missing-alt".to_owned()]);
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule.as_str() == "html/img-missing-alt"),
            "disabled rule should not fire: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn per_slide_disable_leaves_other_rules() {
        let slide = Slide::new("T")
            .with_body(Content::html(r#"<img src="a.png">"#))
            .with_lint_disabled(["pause/empty-step".to_owned()]);
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule.as_str() == "html/img-missing-alt"),
            "unrelated rule should still fire: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn detects_nested_step_as_error() {
        let slide = Slide::new("T").with_body(Content::html(
            r#"<div class="step">a<div class="step">b</div></div>"#,
        ));
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        let nested = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule.as_str() == "html/nested-step")
            .expect("nested-step diagnostic");
        assert_eq!(nested.severity, Severity::Error);
    }

    #[test]
    fn detects_duplicate_part_names() {
        let make_part = |title: &str| Slide {
            kind: SlideKind::Part,
            title: Content::text(title),
            ..Default::default()
        };
        let report = lint(
            &talk_with(vec![make_part("1. Intro"), make_part("2. Intro")]),
            &LintConfig::default(),
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule.as_str() == "structure/duplicate-part-name"),
            "expected duplicate-part-name, got {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn severity_override_is_applied() {
        let slide = Slide::new("T").with_body(Content::html(r#"<img src="a.png">"#));
        let mut config = LintConfig::default();
        config
            .severity_overrides
            .insert("html/img-missing-alt".to_owned(), Severity::Error);
        let report = lint(&talk_with(vec![slide]), &config);
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule.as_str() == "html/img-missing-alt")
            .expect("img-missing-alt diagnostic");
        assert_eq!(diagnostic.severity, Severity::Error);
    }

    #[test]
    fn disabled_rule_is_skipped() {
        let slide = Slide::new("T").with_body(Content::html(r#"<img src="a.png">"#));
        let mut config = LintConfig::default();
        config.disabled.insert("html/img-missing-alt".to_owned());
        let report = lint(&talk_with(vec![slide]), &config);
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule.as_str() == "html/img-missing-alt")
        );
    }
}
