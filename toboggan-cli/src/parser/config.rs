use comrak::Options;
use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};

use crate::parser::FRONT_MATTER_DELIMITER;

/// Get standardized Markdown parsing options
#[must_use]
pub(crate) fn default_options() -> Options<'static> {
    let mut options = Options::default();

    // Enable extensions
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.front_matter_delimiter = Some(FRONT_MATTER_DELIMITER.to_owned());
    options.extension.alerts = true;
    options.extension.subscript = true;
    options.extension.spoiler = true;
    options.extension.greentext = true;
    options.extension.highlight = true;
    options.extension.math_dollars = true;

    options.render.r#unsafe = true;

    options
}

#[must_use]
pub(super) fn default_plugins() -> Plugins<'static> {
    Plugins::default()
}

/// The syntax highlighting themes that actually exist.
///
/// These are the seven `syntect` ships in `ThemeSet::load_defaults()`, and the
/// list is exhaustive: the adapter looks a theme up by name and **panics** on a
/// miss, deep inside comrak, with `no entry found for key`. `--list-themes`
/// used to advertise twenty-two — `Monokai`, `Nord`, `Dracula`, `gruvbox-*`,
/// `OneHalf*`, `ayu-*` — and the README documented one of them, so following
/// the documentation aborted the build.
pub(crate) const AVAILABLE_THEMES: [&str; 7] = [
    "base16-ocean.dark",
    "base16-ocean.light",
    "base16-eighties.dark",
    "base16-mocha.dark",
    "InspiredGitHub",
    "Solarized (dark)",
    "Solarized (light)",
];

/// The theme used when a deck does not choose one.
pub(crate) const DEFAULT_THEME: &str = "base16-ocean.light";

/// Whether `theme` is one the highlighter can actually load.
#[must_use]
pub(crate) fn is_known_theme(theme: &str) -> bool {
    AVAILABLE_THEMES.contains(&theme)
}

#[must_use]
pub(super) fn create_syntax_highlighter(theme: &str) -> SyntectAdapter {
    SyntectAdapterBuilder::new().theme(theme).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = default_options();
        assert!(options.extension.strikethrough);
        assert!(options.extension.table);
        assert!(options.extension.math_dollars);
        assert_eq!(
            options.extension.front_matter_delimiter,
            Some(FRONT_MATTER_DELIMITER.to_owned())
        );
    }

    #[test]
    fn test_default_plugins() {
        let _plugins = default_plugins();
        // Just verify it doesn't panic and returns successfully
    }
}
