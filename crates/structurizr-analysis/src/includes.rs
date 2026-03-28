//! Raw directive facts for `!include` and related value/container metadata.

use crate::span::TextSpan;

/// Records which concrete syntax form supplied a directive value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveValueKind {
    /// An unquoted bare token.
    BareValue,
    /// An identifier node.
    Identifier,
    /// A double-quoted string literal.
    String,
    /// A triple-quoted text block literal.
    TextBlockString,
    /// Any other node kind, stored verbatim.
    Other(String),
}

impl DirectiveValueKind {
    pub(crate) fn from_node_kind(node_kind: &str) -> Self {
        match node_kind {
            "bare_value" => Self::BareValue,
            "identifier" => Self::Identifier,
            "string" => Self::String,
            "text_block_string" => Self::TextBlockString,
            other => Self::Other(other.to_owned()),
        }
    }
}

pub fn normalized_directive_value(
    raw_value: &str,
    value_kind: &DirectiveValueKind,
) -> String {
    match value_kind {
        DirectiveValueKind::String => strip_wrapping(raw_value, "\"", "\"").to_owned(),
        DirectiveValueKind::TextBlockString => {
            strip_wrapping(raw_value, "\"\"\"", "\"\"\"").to_owned()
        }
        DirectiveValueKind::BareValue
        | DirectiveValueKind::Identifier
        | DirectiveValueKind::Other(_) => raw_value.to_owned(),
    }
}

fn strip_wrapping<'a>(value: &'a str, prefix: &str, suffix: &str) -> &'a str {
    value
        .strip_prefix(prefix)
        .and_then(|stripped| stripped.strip_suffix(suffix))
        .unwrap_or(value)
}

/// Records the syntactic block that directly contains a directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveContainer {
    /// The directive appears at the source file root.
    SourceFile,
    /// The directive appears within a `workspace` block.
    Workspace,
    /// The directive appears within a `model` block.
    Model,
    /// The directive appears within another enclosing node kind.
    Other(String),
}

impl DirectiveContainer {
    pub(crate) fn from_enclosing_kind(node_kind: &str) -> Self {
        match node_kind {
            "source_file" => Self::SourceFile,
            "workspace_block" => Self::Workspace,
            "model_block" => Self::Model,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Captures a raw `!include` directive exactly as it appears in one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    /// The original directive value text, including any surrounding quotes.
    pub raw_value: String,
    /// The concrete syntax form used for the directive value.
    pub value_kind: DirectiveValueKind,
    /// The span of the full `!include` directive node.
    pub span: TextSpan,
    /// The span of just the directive's value node.
    pub value_span: TextSpan,
    /// The nearest supported enclosing block for the directive.
    pub container: DirectiveContainer,
}
