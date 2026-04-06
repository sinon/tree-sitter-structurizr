//! Raw string-constant facts extracted from `!const` and `!constant` directives.

use crate::{includes::DirectiveContainer, span::TextSpan};

/// Captures one ordered string-constant definition exactly as it appears in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDefinition {
    /// The normalized constant name, without surrounding quotes.
    pub name: String,
    /// The normalized constant value, without surrounding quotes.
    pub value: String,
    /// The span of the full directive node.
    pub span: TextSpan,
    /// The span of the constant name node.
    pub name_span: TextSpan,
    /// The span of the constant value node.
    pub value_span: TextSpan,
    /// The nearest supported enclosing block for the directive.
    pub container: DirectiveContainer,
}
