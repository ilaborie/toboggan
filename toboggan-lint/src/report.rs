use serde::Serialize;

use crate::diagnostic::{LintDiagnostic, Severity};

/// The result of linting a talk: all diagnostics plus severity counts.
#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    /// All diagnostics, in discovery order.
    pub diagnostics: Vec<LintDiagnostic>,
    /// Number of error-severity diagnostics.
    pub errors: usize,
    /// Number of warning-severity diagnostics.
    pub warnings: usize,
    /// Number of info-severity diagnostics.
    pub infos: usize,
}

impl LintReport {
    /// Builds a report from raw diagnostics, computing severity counts.
    #[must_use]
    pub fn new(diagnostics: Vec<LintDiagnostic>) -> Self {
        let mut errors = 0;
        let mut warnings = 0;
        let mut infos = 0;
        for diagnostic in &diagnostics {
            match diagnostic.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
                Severity::Info => infos += 1,
            }
        }
        Self {
            diagnostics,
            errors,
            warnings,
            infos,
        }
    }

    /// Returns `true` when there are no diagnostics at all.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns the worst severity present, or `None` when clean.
    #[must_use]
    pub fn worst_severity(&self) -> Option<Severity> {
        self.diagnostics
            .iter()
            .map(|diagnostic| diagnostic.severity)
            .max()
    }
}
