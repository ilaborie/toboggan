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
