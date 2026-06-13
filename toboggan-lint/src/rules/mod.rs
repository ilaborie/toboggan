pub(crate) mod content;
pub(crate) mod html;
pub(crate) mod pause;
pub(crate) mod structure;
pub(crate) mod terminal;

#[cfg(feature = "spell")]
pub(crate) mod spelling;

use crate::rule::Rule;

/// Returns every built-in lint rule.
#[must_use]
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
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
        Box::new(content::ExcessiveWords),
        Box::new(content::TooManyImages),
    ]
}
