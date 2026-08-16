use std::collections::{HashMap, HashSet};

use toboggan_core::{Content, Slide, Talk};

use crate::diagnostic::{LintDiagnostic, RuleId, Severity, SlideRef};

/// Thresholds and rule toggles controlling the linter.
#[derive(Debug, Clone)]
pub struct LintConfig {
    /// Rule ids that are disabled.
    pub disabled: HashSet<String>,
    /// Per-rule severity overrides.
    pub severity_overrides: HashMap<String, Severity>,
    /// Maximum reveal steps before `pause/too-many-steps` fires.
    pub max_steps_per_slide: usize,
    /// Maximum words before `content/excessive-words` fires.
    pub max_words_per_slide: usize,
    /// Maximum images before `content/too-many-images` fires.
    pub max_images_per_slide: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            disabled: HashSet::new(),
            severity_overrides: HashMap::new(),
            max_steps_per_slide: 20,
            max_words_per_slide: 120,
            max_images_per_slide: 8,
        }
    }
}

impl LintConfig {
    /// Disables `rule`.
    ///
    /// Takes a [`RuleId`] rather than a string so callers reference
    /// [`crate::ids`] and a renamed rule breaks the build instead of silently
    /// disabling nothing.
    pub fn disable(&mut self, rule: RuleId) -> &mut Self {
        self.disabled.insert(rule.as_str().to_owned());
        self
    }

    /// Returns whether `rule` is enabled.
    #[must_use]
    pub fn is_enabled(&self, rule: RuleId) -> bool {
        !self.disabled.contains(rule.as_str())
    }

    /// Returns the effective severity for `rule`, honoring any override.
    #[must_use]
    pub fn severity(&self, rule: RuleId, default: Severity) -> Severity {
        self.severity_overrides
            .get(rule.as_str())
            .copied()
            .unwrap_or(default)
    }
}

/// Per-slide context passed to [`Rule::check_slide`].
pub struct RuleContext<'a> {
    /// The whole talk (for cross-references such as resolved cwds).
    pub talk: &'a Talk,
    /// The slide under inspection.
    pub slide: &'a Slide,
    /// Reference identifying `slide`.
    pub slide_ref: &'a SlideRef,
    /// Active configuration.
    pub config: &'a LintConfig,
}

/// A lint rule. Most rules implement [`Rule::check_slide`]; cross-slide rules
/// implement [`Rule::check_talk`].
pub trait Rule: Send + Sync {
    /// Stable identifier for this rule.
    fn id(&self) -> RuleId;

    /// Severity used when the config does not override it.
    fn default_severity(&self) -> Severity;

    /// Per-slide check.
    fn check_slide(&self, _context: &RuleContext<'_>, _out: &mut Vec<LintDiagnostic>) {}

    /// Talk-level check (e.g. duplicate part names).
    fn check_talk(&self, _talk: &Talk, _config: &LintConfig, _out: &mut Vec<LintDiagnostic>) {}
}

/// Returns the rendered HTML (or plain text) of a slide body for inspection.
#[must_use]
pub(crate) fn body_html(slide: &Slide) -> &str {
    match &slide.body {
        Content::Html { raw, .. } => raw,
        Content::Text { text } => text,
        Content::Empty => "",
    }
}
