use strz_analysis::{DocumentAnalyzer, DocumentId, DocumentInput};

use crate::{FormatError, FormatOptions, SyntaxErrorPolicy, printer};

// =============================================================================
// Reusable formatter entrypoint
// =============================================================================
//
// This crate intentionally owns a formatter session object rather than a single
// free function so the eventual printer can reuse one `DocumentAnalyzer` across
// repeated calls. That keeps the public surface transport-agnostic while still
// matching the repository's existing snapshot-oriented analysis architecture.

/// Reusable formatter session for Structurizr documents.
pub struct Formatter {
    analyzer: DocumentAnalyzer,
    options: FormatOptions,
}

impl Formatter {
    /// Creates a formatter session with the requested policy.
    #[must_use]
    pub fn new(options: FormatOptions) -> Self {
        Self {
            analyzer: DocumentAnalyzer::new(),
            options,
        }
    }

    /// Returns the active formatter policy.
    #[must_use]
    pub const fn options(&self) -> &FormatOptions {
        &self.options
    }

    /// Formats one Structurizr document.
    ///
    /// # Errors
    ///
    /// Returns [`FormatError::SyntaxErrors`] when the analyzed document contains
    /// parse recovery and the active policy refuses to rewrite it.
    pub fn format_document(
        &mut self,
        input: DocumentInput,
    ) -> Result<FormattedDocument, FormatError> {
        let snapshot = self.analyzer.analyze(input);

        if snapshot.has_syntax_errors()
            && matches!(self.options.syntax_errors(), SyntaxErrorPolicy::Refuse)
        {
            return Err(FormatError::SyntaxErrors {
                diagnostics: snapshot.syntax_diagnostics().to_vec(),
            });
        }

        let formatted = printer::format_source(snapshot.source(), &self.options);
        Ok(FormattedDocument::new(
            snapshot.id().clone(),
            formatted.clone(),
            formatted != snapshot.source(),
        ))
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new(FormatOptions::default())
    }
}

/// One formatter result for a single physical document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedDocument {
    id: DocumentId,
    formatted: String,
    changed: bool,
}

impl FormattedDocument {
    /// Creates one formatter result from the target document id and rendered source.
    #[must_use]
    pub fn new(id: DocumentId, formatted: impl Into<String>, changed: bool) -> Self {
        Self {
            id,
            formatted: formatted.into(),
            changed,
        }
    }

    /// Returns the stable caller-provided document identifier.
    #[must_use]
    pub const fn id(&self) -> &DocumentId {
        &self.id
    }

    /// Returns the formatter output source text.
    #[must_use]
    pub fn formatted(&self) -> &str {
        &self.formatted
    }

    /// Returns whether the formatter changed the input bytes.
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.changed
    }

    /// Consumes the result and returns the formatted source text.
    #[must_use]
    pub fn into_formatted(self) -> String {
        self.formatted
    }
}
