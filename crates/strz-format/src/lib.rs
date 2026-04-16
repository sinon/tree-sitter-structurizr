#![warn(missing_docs)]
//! Transport-agnostic formatting primitives for Structurizr DSL documents.
//!
//! This crate sits above [`strz_analysis`] and below CLI/editor integration. Its
//! job is to own formatting policy, formatter-session orchestration, and the
//! eventual CST-aware printer without pushing syntax-aware layout logic into the
//! outer transport layer.

mod error;
mod formatter;
mod options;
mod printer;

pub use error::FormatError;
pub use formatter::{FormattedDocument, Formatter};
pub use options::{
    CommentFormatPolicy, FormatOptions, Indentation, LineWidthPolicy, OverflowPolicy,
    SpacingPolicy, SyntaxErrorPolicy,
};

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::{
        CommentFormatPolicy, FormatError, FormatOptions, Formatter, OverflowPolicy,
        SyntaxErrorPolicy,
    };
    use strz_analysis::DocumentInput;

    #[test]
    fn default_options_match_the_locked_formatter_policy() {
        let options = FormatOptions::default();

        assert_eq!(options.indentation().width(), 4);
        assert_eq!(options.line_width().target(), 110);
        assert_eq!(
            options.line_width().comments(),
            CommentFormatPolicy::Preserve
        );
        assert_eq!(options.line_width().overflow(), OverflowPolicy::BestEffort);
        assert_eq!(options.spacing().top_level_gap(), 1);
        assert_eq!(options.spacing().sibling_block_gap(), 1);
        assert_eq!(options.syntax_errors(), SyntaxErrorPolicy::Refuse);
    }

    #[test]
    fn parse_error_documents_are_rejected_before_printing() {
        let mut formatter = Formatter::default();
        let error = formatter
            .format_document(DocumentInput::new("workspace.dsl", "workspace {"))
            .expect_err("syntax-error documents should be rejected");

        match error {
            FormatError::SyntaxErrors { diagnostics } => {
                assert!(
                    !diagnostics.is_empty(),
                    "syntax-error rejection should surface the blocking diagnostics"
                );
            }
        }
    }

    #[test]
    fn clean_documents_return_real_formatted_output() {
        let mut formatter = Formatter::default();
        let result = formatter
            .format_document(DocumentInput::new(
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                    model {
                    user = person "User"
                    }
                    }
                "#},
            ))
            .expect("clean documents should format successfully");

        assert_eq!(
            result.formatted(),
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                    }
                }
            "#}
        );
        assert!(result.changed());
    }
}
