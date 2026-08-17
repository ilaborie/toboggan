pub(crate) mod code;
pub(crate) mod content;
pub(crate) mod html;
pub(crate) mod pause;
pub(crate) mod structure;
pub(crate) mod terminal;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(feature = "spell")]
pub(crate) mod spelling;

use crate::diagnostic::RuleId;
use crate::rule::Rule;

/// The id of every built-in rule.
///
/// Rules return these from `Rule::id()` and callers that need to name a rule
/// (`--no-spell`, the MCP `lint` tool) reference them instead of retyping the
/// string. Renaming a rule is then a compile error at every use site rather than
/// a silently-ignored disable directive.
pub mod ids {
    use crate::diagnostic::RuleId;

    /// `<!-- pause -->` used inside a part slide.
    pub const PAUSE_IN_PART: RuleId = RuleId("pause/in-part");
    /// A reveal step with no content.
    pub const PAUSE_EMPTY_STEP: RuleId = RuleId("pause/empty-step");
    /// More reveal steps than `max_steps_per_slide`.
    pub const PAUSE_TOO_MANY_STEPS: RuleId = RuleId("pause/too-many-steps");
    /// A terminal directive inside a part slide.
    pub const TERM_IN_PART: RuleId = RuleId("term/in-part");
    /// A terminal cwd that does not resolve on disk.
    pub const TERM_UNRESOLVED_CWD: RuleId = RuleId("term/unresolved-cwd");
    /// Two terminals sharing one cwd.
    pub const TERM_DUPLICATE_CWD: RuleId = RuleId("term/duplicate-cwd");
    /// A reveal step nested inside another.
    pub const HTML_NESTED_STEP: RuleId = RuleId("html/nested-step");
    /// An `<img>` without an `alt` attribute.
    pub const HTML_IMG_MISSING_ALT: RuleId = RuleId("html/img-missing-alt");
    /// A raw `<script>` in slide content.
    pub const HTML_RAW_SCRIPT: RuleId = RuleId("html/raw-script");
    /// An `<h1>` in the body of a slide that already renders a title.
    pub const HTML_HEADING_H1: RuleId = RuleId("html/heading-h1");
    /// A slide with no content.
    pub const STRUCTURE_EMPTY_SLIDE: RuleId = RuleId("structure/empty-slide");
    /// A slide with no title.
    pub const STRUCTURE_TITLE_MISSING: RuleId = RuleId("structure/title-missing");
    /// Two parts sharing a name.
    pub const STRUCTURE_DUPLICATE_PART_NAME: RuleId = RuleId("structure/duplicate-part-name");
    /// Two content slides sharing a title.
    pub const STRUCTURE_DUPLICATE_SLIDE_TITLE: RuleId = RuleId("structure/duplicate-slide-title");
    /// A code block longer than `max_code_lines`.
    pub const CODE_TOO_LONG: RuleId = RuleId("code/too-long");
    /// A code block with no language on its fence.
    pub const CODE_NO_LANGUAGE: RuleId = RuleId("code/no-language");
    /// More words than `max_words_per_slide`.
    pub const CONTENT_EXCESSIVE_WORDS: RuleId = RuleId("content/excessive-words");
    /// More images than `max_images_per_slide`.
    pub const CONTENT_TOO_MANY_IMAGES: RuleId = RuleId("content/too-many-images");
    /// A typo reported by the `typos` CLI.
    pub const SPELLING_TYPO: RuleId = RuleId("spelling/typo");
}

/// Every rule id known to this build, for validating user-supplied ids.
#[must_use]
pub fn all_rule_ids() -> Vec<RuleId> {
    all_rules().iter().map(|rule| rule.id()).collect()
}

/// Returns every built-in lint rule.
#[must_use]
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = vec![
        Box::new(pause::PauseInPart),
        Box::new(pause::EmptyStep),
        Box::new(pause::TooManySteps),
        Box::new(terminal::TerminalInPart),
        Box::new(terminal::UnresolvedCwd),
        Box::new(terminal::DuplicateCwd),
        Box::new(html::NestedStep),
        Box::new(html::ImgMissingAlt),
        Box::new(html::RawScript),
        Box::new(html::HeadingH1),
        Box::new(structure::EmptySlide),
        Box::new(structure::TitleMissing),
        Box::new(structure::DuplicatePartName),
        Box::new(structure::DuplicateSlideTitle),
        Box::new(content::ExcessiveWords),
        Box::new(content::TooManyImages),
        Box::new(code::CodeTooLong),
        Box::new(code::CodeNoLanguage),
    ];
    // Spell checking is opt-in at compile time (it needs the `typos` CLI at
    // runtime). Disable it for a run with `--no-spell`; it cannot be silenced by
    // a slide's `disabled_rules` front matter or a `<!-- lint-disable -->`
    // directive, because it is a talk-level rule and per-slide directives only
    // guard `check_slide`.
    #[cfg(feature = "spell")]
    rules.push(Box::new(spelling::SpellCheck));
    rules
}
