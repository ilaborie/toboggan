//! Lints for Toboggan presentations.
//!
//! The library runs a set of [`Rule`]s over a parsed [`Talk`] and produces a
//! framework-neutral [`LintReport`]. It deliberately has no CLI/terminal
//! dependencies, so the consumers decide on presentation: the `toboggan` binary
//! prints coloured lines, and `toboggan-mcp` serializes the report to JSON.
#![warn(missing_docs)]

mod diagnostic;
mod report;
mod rule;
pub mod rules;

use toboggan_core::Talk;

use self::diagnostic::SlideRef as Ref;
pub use self::diagnostic::{LintDiagnostic, RuleId, Severity, SlideRef};
pub use self::report::LintReport;
pub use self::rule::{LintConfig, Rule, RuleContext};
pub use self::rules::{all_rule_ids, all_rules, ids};

/// Reported when a disable directive names a rule that does not exist.
///
/// Not in [`ids`] because no `Rule` produces it: it is the linter reporting on
/// its own configuration.
const UNKNOWN_RULE: &str = "lint/unknown-rule";

/// Lints `talk` with the given configuration and returns a [`LintReport`].
#[must_use]
pub fn lint(talk: &Talk, config: &LintConfig) -> LintReport {
    let rules = all_rules();
    let mut out = Vec::new();

    // A rule id nobody recognises silences nothing, so a typo in `disabled_rules`
    // or a `<!-- lint-disable -->` directive would otherwise leave the author
    // believing a rule was off. Reported as a diagnostic rather than logged: this
    // library has no logger, and the author is the one who can fix it.
    let known = all_rule_ids();
    let is_known = |id: &str| known.iter().any(|rule| rule.as_str() == id);
    for id in config.disabled.iter().filter(|id| !is_known(id)) {
        out.push(LintDiagnostic::talk(
            RuleId(UNKNOWN_RULE),
            Severity::Warning,
            format!("unknown lint rule id `{id}` in the disabled list"),
        ));
    }
    for slide in &talk.slides {
        for id in slide.lint_disabled.iter().filter(|id| !is_known(id)) {
            let mut diagnostic = LintDiagnostic::talk(
                RuleId(UNKNOWN_RULE),
                Severity::Warning,
                format!(
                    "unknown lint rule id `{id}` disabled on slide \"{}\"",
                    slide.title
                ),
            );
            diagnostic.source_path.clone_from(&slide.source_path);
            out.push(diagnostic);
        }
    }

    // Talk-level rules first.
    for rule in &rules {
        if config.is_enabled(rule.id()) {
            rule.check_talk(talk, config, &mut out);
        }
    }

    // Per-slide rules.
    for (index, slide) in talk.slides.iter().enumerate() {
        let slide_ref = Ref::new(index, slide);
        let context = RuleContext::new(talk, slide, &slide_ref, config);
        let before = out.len();
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
        // Everything just pushed came from this slide, so it came from this
        // slide's file. Stamped centrally rather than in each rule: a rule that
        // forgot would silently produce a diagnostic nobody can locate.
        for diagnostic in out.iter_mut().skip(before) {
            diagnostic.source_path.clone_from(&slide.source_path);
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
    use std::path::{Path, PathBuf};

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
        assert_eq!(report.warnings(), 1);
        assert_eq!(report.errors(), 0);
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
    /// Finds the ids of every diagnostic in `report`.
    fn ids_in(report: &LintReport) -> Vec<&str> {
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule.as_str())
            .collect()
    }

    fn fires(report: &LintReport, rule: RuleId) -> bool {
        ids_in(report).contains(&rule.as_str())
    }

    /// Every declared id must have a rule behind it.
    ///
    /// `all_rules` is a hand-written `vec![]`, so dropping one `Box::new` line
    /// stops that rule running on every deck and leaves the whole suite green —
    /// each rule's own tests call it directly, and nothing checked that the
    /// runner still reached it.
    ///
    /// `spelling/typo` is deliberately absent: it is registered only under the
    /// `spell` feature, so its id is not in `DECLARED_IDS`.
    #[test]
    fn registered_rules_cover_every_id() {
        let registered = all_rule_ids()
            .into_iter()
            .map(RuleId::as_str)
            .collect::<Vec<_>>();

        for declared in rules::DECLARED_IDS {
            assert!(
                registered.contains(&declared.as_str()),
                "{} is declared but no rule is registered for it",
                declared.as_str()
            );
        }
    }

    /// The reverse: a rule whose id was never declared cannot be referenced by
    /// `--no-spell`, a `disabled_rules` entry, or the MCP tool.
    #[test]
    fn every_registered_rule_has_a_declared_id() {
        let declared = rules::DECLARED_IDS
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>();

        for rule in all_rule_ids() {
            // Skip the feature-gated one, which is registered without being
            // listed above.
            if rule.as_str() == ids::SPELLING_TYPO.as_str() {
                continue;
            }
            assert!(
                declared.contains(&rule.as_str()),
                "{} runs but is not declared in `ids`",
                rule.as_str()
            );
        }
    }

    /// Every rule must be reachable: a rule that no input can trigger is dead
    /// weight, and one whose id is misspelled can never be disabled.
    #[test]
    fn every_rule_id_is_unique() {
        let mut ids = all_rule_ids()
            .into_iter()
            .map(RuleId::as_str)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(
            ids.len(),
            count,
            "duplicate rule ids would be undisableable"
        );
    }

    #[test]
    fn detects_empty_step() {
        let slide = Slide::new("T").with_body(Content::html(r#"<div class="step"></div>"#));
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        assert!(
            fires(&report, ids::PAUSE_EMPTY_STEP),
            "{:?}",
            ids_in(&report)
        );
    }

    #[test]
    fn detects_raw_script() {
        let slide = Slide::new("T").with_body(Content::html("<script>alert(1)</script>"));
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        assert!(
            fires(&report, ids::HTML_RAW_SCRIPT),
            "{:?}",
            ids_in(&report)
        );
    }

    #[test]
    fn detects_empty_slide() {
        let report = lint(&talk_with(vec![Slide::default()]), &LintConfig::default());
        assert!(
            fires(&report, ids::STRUCTURE_EMPTY_SLIDE),
            "{:?}",
            ids_in(&report)
        );
    }

    #[test]
    fn detects_missing_title_on_a_part() {
        let part = Slide {
            kind: SlideKind::Part,
            body: Content::html("<p>x</p>"),
            ..Default::default()
        };
        let report = lint(&talk_with(vec![part]), &LintConfig::default());
        assert!(
            fires(&report, ids::STRUCTURE_TITLE_MISSING),
            "{:?}",
            ids_in(&report)
        );
    }

    /// The three `max_*` thresholds are `>` comparisons, so a slide sitting
    /// exactly on the limit must stay quiet. A `>` → `>=` slip is invisible
    /// without this pair of assertions.
    #[test]
    fn excessive_words_fires_only_above_the_limit() {
        let config = LintConfig {
            max_words_per_slide: 5,
            ..LintConfig::default()
        };
        // The title counts toward the total, so a 1-word title plus 4 body words
        // sits exactly on the limit of 5.
        let at_limit = Slide::new("T").with_body(Content::html("<p>one two three four</p>"));
        let report = lint(&talk_with(vec![at_limit]), &config);
        assert!(
            !fires(&report, ids::CONTENT_EXCESSIVE_WORDS),
            "{:?}",
            ids_in(&report)
        );

        let over = Slide::new("T").with_body(Content::html("<p>one two three four five</p>"));
        let report = lint(&talk_with(vec![over]), &config);
        assert!(
            fires(&report, ids::CONTENT_EXCESSIVE_WORDS),
            "{:?}",
            ids_in(&report)
        );
    }

    #[test]
    fn too_many_images_fires_only_above_the_limit() {
        let config = LintConfig {
            max_images_per_slide: 2,
            ..LintConfig::default()
        };
        let img = r#"<img src="a.png" alt="a">"#;
        let at_limit = Slide::new("T").with_body(Content::html(format!("{img}{img}")));
        let report = lint(&talk_with(vec![at_limit]), &config);
        assert!(
            !fires(&report, ids::CONTENT_TOO_MANY_IMAGES),
            "{:?}",
            ids_in(&report)
        );

        let over = Slide::new("T").with_body(Content::html(format!("{img}{img}{img}")));
        let report = lint(&talk_with(vec![over]), &config);
        assert!(
            fires(&report, ids::CONTENT_TOO_MANY_IMAGES),
            "{:?}",
            ids_in(&report)
        );
    }

    #[test]
    fn too_many_steps_fires_only_above_the_limit() {
        let config = LintConfig {
            max_steps_per_slide: 2,
            ..LintConfig::default()
        };
        let step = r#"<div class="step">x</div>"#;
        let at_limit = Slide::new("T").with_body(Content::html(format!("{step}{step}")));
        let report = lint(&talk_with(vec![at_limit]), &config);
        assert!(
            !fires(&report, ids::PAUSE_TOO_MANY_STEPS),
            "{:?}",
            ids_in(&report)
        );

        let over = Slide::new("T").with_body(Content::html(format!("{step}{step}{step}")));
        let report = lint(&talk_with(vec![over]), &config);
        assert!(
            fires(&report, ids::PAUSE_TOO_MANY_STEPS),
            "{:?}",
            ids_in(&report)
        );
    }

    /// A rule id nobody recognises silences nothing, so it has to be reported —
    /// otherwise a typo leaves the author believing a rule is off.
    #[test]
    fn unknown_disabled_rule_id_is_reported() {
        let mut config = LintConfig::default();
        config.disabled.insert("html/img-missing-alts".to_owned());
        let slide = Slide::new("T").with_body(Content::html(r#"<img src="a.png">"#));
        let report = lint(&talk_with(vec![slide]), &config);
        assert!(
            ids_in(&report).contains(&"lint/unknown-rule"),
            "expected an unknown-rule diagnostic, got {:?}",
            ids_in(&report)
        );
        // ...and the real rule still fires, because nothing was silenced.
        assert!(
            fires(&report, ids::HTML_IMG_MISSING_ALT),
            "{:?}",
            ids_in(&report)
        );
    }

    /// The whole point of `source_path`: a reader has to be able to open the
    /// file a diagnostic is about. Rules never set it — `lint` stamps it — so
    /// this covers every rule at once.
    #[test]
    fn a_slide_diagnostic_names_the_file_it_came_from() {
        let slide = Slide::new("T")
            .with_body(Content::html(r#"<img src="a.png">"#))
            .with_source_path("slides/1_intro/2-hello.md");
        let report = lint(&talk_with(vec![slide]), &LintConfig::default());
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule == ids::HTML_IMG_MISSING_ALT)
            .expect("img-missing-alt diagnostic");
        assert_eq!(
            diagnostic.source_path.as_deref(),
            Some(Path::new("slides/1_intro/2-hello.md"))
        );
    }

    /// A slide with no file of its own — the implicit part slide a folder gets
    /// without a `_part.md`, or a talk deserialized from a built artifact —
    /// must not borrow a neighbour's path.
    #[test]
    fn a_slide_without_a_file_gets_no_path() {
        let with_file = Slide::new("A")
            .with_body(Content::html(r#"<img src="a.png">"#))
            .with_source_path("slides/a.md");
        let without = Slide::new("B").with_body(Content::html(r#"<img src="b.png">"#));
        let report = lint(&talk_with(vec![with_file, without]), &LintConfig::default());
        let paths = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule == ids::HTML_IMG_MISSING_ALT)
            .map(|diagnostic| diagnostic.source_path.clone())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec![Some(PathBuf::from("slides/a.md")), None]);
    }

    /// `disable` takes a `RuleId`, so this cannot drift from the rule's own id.
    #[test]
    fn disable_by_rule_id_silences_the_rule() {
        let mut config = LintConfig::default();
        config.disable(ids::HTML_IMG_MISSING_ALT);
        let slide = Slide::new("T").with_body(Content::html(r#"<img src="a.png">"#));
        let report = lint(&talk_with(vec![slide]), &config);
        assert!(
            !fires(&report, ids::HTML_IMG_MISSING_ALT),
            "{:?}",
            ids_in(&report)
        );
    }
}
