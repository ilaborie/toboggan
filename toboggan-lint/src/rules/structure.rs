use std::collections::HashMap;

use toboggan_core::{Content, SlideKind, Talk};
use toboggan_stats::strip_slide_counter;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{LintConfig, Rule, RuleContext};

/// A slide with no title, body, or notes.
pub(crate) struct EmptySlide;

impl Rule for EmptySlide {
    fn id(&self) -> RuleId {
        RuleId("structure/empty-slide")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let slide = context.slide;
        let empty = matches!(slide.title, Content::Empty)
            && matches!(slide.body, Content::Empty)
            && matches!(slide.notes, Content::Empty);
        if empty {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "slide has no title, body, or notes",
                )
                .with_help("add content or remove the slide"),
            );
        }
    }
}

/// Cover and part slides should have a title.
pub(crate) struct TitleMissing;

impl Rule for TitleMissing {
    fn id(&self) -> RuleId {
        RuleId("structure/title-missing")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let needs_title = matches!(context.slide.kind, SlideKind::Cover | SlideKind::Part);
        if needs_title && matches!(context.slide.title, Content::Empty) {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{:?} slide has no title", context.slide.kind),
                )
                .with_help("set a `title` in the slide front matter"),
            );
        }
    }
}

/// Two part (section) slides share the same name.
pub(crate) struct DuplicatePartName;

impl Rule for DuplicatePartName {
    fn id(&self) -> RuleId {
        RuleId("structure/duplicate-part-name")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_talk(&self, talk: &Talk, _config: &LintConfig, out: &mut Vec<LintDiagnostic>) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for slide in &talk.slides {
            if slide.kind != SlideKind::Part {
                continue;
            }
            if let Content::Empty = slide.title {
                continue;
            }
            let name = strip_slide_counter(&slide.title.to_string())
                .trim()
                .to_owned();
            *counts.entry(name).or_insert(0) += 1;
        }

        let mut duplicates = counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        duplicates.sort();

        for name in duplicates {
            out.push(
                LintDiagnostic::talk(
                    self.id(),
                    self.default_severity(),
                    format!("duplicate part name: \"{name}\""),
                )
                .with_help("give each section a unique title (stats group sections by name)"),
            );
        }
    }
}
