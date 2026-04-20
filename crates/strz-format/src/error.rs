use std::fmt;

use strz_analysis::RuledDiagnostic;

// =============================================================================
// Formatter failure surface
// =============================================================================
//
// The formatter has two qualitatively different failure modes from day one:
//
// 1. Some inputs are intentionally out of scope because syntax recovery makes the
//    tree shape unreliable.
// 2. The crate skeleton exists before the printer implementation lands, so clean
//    documents need an explicit transitional error rather than a silent
//    no-op/"already formatted" lie.

/// One formatter failure surfaced by the transport-agnostic formatting core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The document contains syntax recovery, so v1 refuses to rewrite it.
    SyntaxErrors {
        /// The syntax diagnostics that blocked formatting.
        diagnostics: Vec<RuledDiagnostic>,
    },
}

impl FormatError {
    /// Returns the blocking syntax diagnostics when formatting was refused for a
    /// parse-error document.
    #[must_use]
    pub fn syntax_diagnostics(&self) -> Option<&[RuledDiagnostic]> {
        match self {
            Self::SyntaxErrors { diagnostics } => Some(diagnostics),
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyntaxErrors { diagnostics } => write!(
                formatter,
                "{} syntax diagnostic(s) prevent formatting",
                diagnostics.len()
            ),
        }
    }
}

impl std::error::Error for FormatError {}
