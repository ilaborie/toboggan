use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use toboggan_core::{Content, Slide, SlideId};

/// Severity of a lint diagnostic.
///
/// Ordered so that `Error` is the greatest — `Iterator::max` over severities
/// yields the worst one.
///
/// `Deserialize` as well as `Serialize`: severities arrive *into* the linter
/// from a `toboggan.toml` `[lint.severity]` table, not just out of it in a
/// report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational suggestion.
    Info,
    /// A likely problem.
    #[default]
    Warning,
    /// A definite problem (breaks rendering or output).
    Error,
}

/// Stable machine-readable rule identifier, e.g. `"pause/in-part"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RuleId(pub &'static str);

impl RuleId {
    /// The underlying identifier string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Identifies the slide a diagnostic refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlideRef {
    /// Zero-based index into `talk.slides` (matches `/api/slides`).
    pub index: usize,
    /// One-based number for display.
    pub display_number: usize,
    /// Best-effort title text for the slide.
    pub title: String,
}

impl SlideRef {
    /// Builds a [`SlideRef`] for the slide at `index`.
    #[must_use]
    pub fn new(index: usize, slide: &Slide) -> Self {
        Self {
            index,
            display_number: SlideId::new(index).display_number(),
            title: slide_title(slide),
        }
    }
}

/// A single lint finding. Framework-neutral: the `toboggan` binary prints it
/// with `owo_colors`, and the MCP server serializes it to JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintDiagnostic {
    /// The rule that produced this diagnostic.
    pub rule: RuleId,
    /// How serious the finding is.
    pub severity: Severity,
    /// The slide it refers to, or `None` for talk-level findings.
    pub slide: Option<SlideRef>,
    /// What is wrong.
    pub message: String,
    /// How to fix it, when known.
    pub help: Option<String>,
    /// Source file the finding originates from, when known.
    pub source_path: Option<PathBuf>,
}

impl LintDiagnostic {
    /// Creates a slide-level diagnostic.
    #[must_use]
    pub fn slide(
        rule: RuleId,
        severity: Severity,
        slide: &SlideRef,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            severity,
            slide: Some(slide.clone()),
            message: message.into(),
            help: None,
            source_path: None,
        }
    }

    /// Creates a talk-level diagnostic (not tied to a single slide).
    #[must_use]
    pub fn talk(rule: RuleId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule,
            severity,
            slide: None,
            message: message.into(),
            help: None,
            source_path: None,
        }
    }

    /// Attaches a help string describing how to fix the finding.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Attaches the source file the finding refers to.
    ///
    /// Rules do not call this: [`crate::lint`] stamps every diagnostic a slide
    /// produced with that slide's file, so a rule cannot forget to.
    #[must_use]
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }
}

/// Best-effort plain-text title for a slide.
fn slide_title(slide: &Slide) -> String {
    match &slide.title {
        Content::Empty => match &slide.body {
            Content::Empty => String::new(),
            other => first_line(&other.to_string()),
        },
        other => first_line(&other.to_string()),
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_owned()
}
