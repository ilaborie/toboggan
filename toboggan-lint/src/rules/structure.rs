use std::collections::HashMap;

use toboggan_core::{Content, SlideKind, Talk};
use toboggan_stats::strip_slide_counter;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity, SlideRef};
use crate::rule::{LintConfig, Rule, RuleContext};

/// A slide with no title, body, or notes.
pub(crate) struct EmptySlide;

impl Rule for EmptySlide {
    fn id(&self) -> RuleId {
        super::ids::STRUCTURE_EMPTY_SLIDE
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
        super::ids::STRUCTURE_TITLE_MISSING
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
        super::ids::STRUCTURE_DUPLICATE_PART_NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_talk(&self, talk: &Talk, _config: &LintConfig, out: &mut Vec<LintDiagnostic>) {
        for (name, _) in repeated_titles(talk, SlideKind::Part) {
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

/// Two content slides share the same title.
///
/// Only parts were checked before, but a repeated slide title is the more
/// common mistake: it is what a copied-and-pasted slide looks like, and it
/// makes the overview page, the TUI slide list and every `#slide-N` deep link
/// ambiguous to anyone reading them.
pub(crate) struct DuplicateSlideTitle;

impl Rule for DuplicateSlideTitle {
    fn id(&self) -> RuleId {
        super::ids::STRUCTURE_DUPLICATE_SLIDE_TITLE
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    /// Reports each offending slide rather than each repeated title.
    ///
    /// The finding is inherently cross-slide, so it has to run here — but a
    /// talk-level diagnostic has no slide, and therefore no file, and "2 slides
    /// share the title X" leaves the author to find both. One diagnostic per
    /// slide names each file instead.
    ///
    /// The runner only applies per-slide `lint-disable` directives to
    /// `check_slide`, so this honors them itself; otherwise a deliberate repeat
    /// could only be silenced for the whole deck.
    fn check_talk(&self, talk: &Talk, _config: &LintConfig, out: &mut Vec<LintDiagnostic>) {
        let repeated: HashMap<String, usize> = repeated_titles(talk, SlideKind::Standard)
            .into_iter()
            .collect();
        if repeated.is_empty() {
            return;
        }

        for (index, slide) in talk.slides.iter().enumerate() {
            if slide.kind != SlideKind::Standard {
                continue;
            }
            if slide
                .lint_disabled
                .iter()
                .any(|disabled| disabled == self.id().as_str())
            {
                continue;
            }
            let title = strip_slide_counter(&slide.title.to_string())
                .trim()
                .to_owned();
            let Some(count) = repeated.get(&title) else {
                continue;
            };
            let others = count.saturating_sub(1);
            let plural = if others == 1 { "slide" } else { "slides" };
            let mut diagnostic = LintDiagnostic::slide(
                self.id(),
                self.default_severity(),
                &SlideRef::new(index, slide),
                format!("title \"{title}\" is shared with {others} other {plural}"),
            )
            .with_help("give each slide a distinct title, or continue one with \"(cont.)\"");
            diagnostic.source_path.clone_from(&slide.source_path);
            out.push(diagnostic);
        }
    }
}

/// Titles held by more than one slide of `kind`, sorted, with their counts.
///
/// Counters are stripped first: the parser prefixes titles with `1.2 `, so two
/// copies of the same slide never share a raw title even when the author sees
/// the same words twice.
fn repeated_titles(talk: &Talk, kind: SlideKind) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for slide in &talk.slides {
        if slide.kind != kind {
            continue;
        }
        if let Content::Empty = slide.title {
            continue;
        }
        let name = strip_slide_counter(&slide.title.to_string())
            .trim()
            .to_owned();
        if name.is_empty() {
            continue;
        }
        *counts.entry(name).or_insert(0) += 1;
    }

    let mut duplicates = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<Vec<_>>();
    duplicates.sort();
    duplicates
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use toboggan_core::Slide;

    use super::*;
    use crate::rules::test_support::{fires, only, talk_diagnostics, talk_of};

    fn standard(title: &str) -> Slide {
        Slide::new(title)
    }

    fn part(title: &str) -> Slide {
        Slide::part(title)
    }

    #[test]
    fn an_empty_slide_is_reported() {
        assert!(fires(&EmptySlide, &Slide::default()));
    }

    /// Notes alone are content: a slide that exists only to carry a speaker
    /// note is unusual but deliberate, and not empty.
    #[test]
    fn notes_alone_are_not_empty() {
        let slide = Slide {
            notes: Content::text("say this"),
            ..Default::default()
        };
        assert!(!fires(&EmptySlide, &slide));
    }

    #[test]
    fn a_cover_and_a_part_need_titles() {
        let untitled_body = Content::html("<p>x</p>");
        for kind in [SlideKind::Cover, SlideKind::Part] {
            let slide = Slide {
                kind,
                body: untitled_body.clone(),
                ..Default::default()
            };
            assert!(fires(&TitleMissing, &slide), "{kind:?} should need a title");
        }
    }

    /// A content slide without a title is ordinary — a full-bleed image, a
    /// quote — so the rule must not fire on one.
    #[test]
    fn a_standard_slide_does_not_need_one() {
        let slide = Slide {
            body: Content::html("<p>x</p>"),
            ..Default::default()
        };
        assert!(!fires(&TitleMissing, &slide));
    }

    #[test]
    fn duplicate_part_names_are_reported_once_each() {
        let talk = talk_of(&[part("1. Intro"), part("2. Intro"), part("3. Intro")]);
        let diagnostics = talk_diagnostics(&DuplicatePartName, &talk);
        assert!(only(&diagnostics).message.contains("Intro"));
    }

    /// The counter prefix is what makes this rule non-trivial: the parser
    /// numbers titles, so two copies of one slide never share a raw string.
    #[test]
    fn the_counter_prefix_is_stripped_before_comparing() {
        let talk = talk_of(&[standard("1.1 Setup"), standard("2.4 Setup")]);
        let diagnostics = talk_diagnostics(&DuplicateSlideTitle, &talk);
        assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
        for diagnostic in &diagnostics {
            assert!(
                diagnostic.message.contains("1 other slide"),
                "{}",
                diagnostic.message
            );
        }
    }

    /// Each offending slide is named, so the author can open both — a single
    /// talk-level "2 slides share X" would leave them hunting.
    #[test]
    fn every_slide_sharing_a_title_is_named_with_its_file() {
        let talk = talk_of(&[
            standard("1.1 Setup").with_source_path("slides/a.md"),
            standard("1.2 Other"),
            standard("2.4 Setup").with_source_path("slides/b.md"),
        ]);
        let paths = talk_diagnostics(&DuplicateSlideTitle, &talk)
            .into_iter()
            .filter_map(|diagnostic| diagnostic.source_path)
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![PathBuf::from("slides/a.md"), PathBuf::from("slides/b.md")]
        );
    }

    /// The runner only applies per-slide disable directives to `check_slide`,
    /// so this cross-slide rule has to honor them itself — otherwise a
    /// deliberate repeat could only be silenced for the whole deck.
    #[test]
    fn a_slide_can_silence_the_rule_for_itself() {
        let talk = talk_of(&[
            standard("1.1 Setup")
                .with_lint_disabled(["structure/duplicate-slide-title".to_owned()]),
            standard("2.4 Setup"),
        ]);
        let diagnostics = talk_diagnostics(&DuplicateSlideTitle, &talk);
        let slide = only(&diagnostics).slide.as_ref().expect("a slide");
        assert_eq!(slide.index, 1, "only the un-silenced slide is reported");
    }

    #[test]
    fn distinct_slide_titles_are_quiet() {
        let talk = talk_of(&[standard("1.1 Setup"), standard("1.2 Teardown")]);
        assert!(talk_diagnostics(&DuplicateSlideTitle, &talk).is_empty());
    }

    /// Parts and slides are counted separately: a part named "Setup"
    /// introducing a slide named "Setup" is a normal deck, not a duplicate.
    #[test]
    fn a_part_and_a_slide_may_share_a_name() {
        let talk = talk_of(&[part("1. Setup"), standard("1.1 Setup")]);
        assert!(talk_diagnostics(&DuplicateSlideTitle, &talk).is_empty());
        assert!(talk_diagnostics(&DuplicatePartName, &talk).is_empty());
    }

    /// Untitled slides fall together under the empty string; reporting them as
    /// duplicates of each other would be noise, and `structure/title-missing`
    /// already covers the cases that matter.
    #[test]
    fn untitled_slides_are_not_duplicates_of_each_other() {
        let blank = Slide {
            body: Content::html("<p>x</p>"),
            ..Default::default()
        };
        let talk = talk_of(&[blank.clone(), blank]);
        assert!(talk_diagnostics(&DuplicateSlideTitle, &talk).is_empty());
    }
}
