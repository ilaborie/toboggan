use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Terminal color theme.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Light text on a dark background. The default, because a slide deck
    /// usually is.
    #[default]
    Dark,
    /// Dark text on a light background, for a deck with a light theme.
    Light,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip_serializing_if requires &T
fn is_dark_theme(theme: &Theme) -> bool {
    *theme == Theme::Dark
}

/// Configuration for an embedded terminal in a slide.
///
/// Parsed from `<!-- term: path/to/cwd -->`, `<!-- term: path/to/cwd :light -->`,
/// `<!-- term: path/to/cwd | command -->`, or `<!-- term: path/to/cwd :light | command -->`.
/// Multiple terminals per slide are supported (rendered side by side).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TerminalConfig {
    /// Working directory for the terminal session
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub cwd: PathBuf,
    /// Terminal color theme, defaults to dark
    #[serde(default, skip_serializing_if = "is_dark_theme")]
    pub theme: Theme,
    /// Optional command to run instead of the default shell
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
}

impl TerminalConfig {
    /// A terminal in `cwd`, with the dark theme and the default shell.
    #[must_use]
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            theme: Theme::default(),
            cmd: None,
        }
    }

    /// Sets the colour theme.
    #[must_use]
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Runs `cmd` on connect instead of dropping into an interactive shell.
    #[must_use]
    pub fn with_cmd(mut self, cmd: impl Into<String>) -> Self {
        self.cmd = Some(cmd.into());
        self
    }
}
