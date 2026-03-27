//! Raw directive facts for `!include` and related value/container metadata.

use crate::span::TextSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveValueKind {
    BareValue,
    Identifier,
    String,
    TextBlockString,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveContainer {
    SourceFile,
    Workspace,
    Model,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub raw_value: String,
    pub value_kind: DirectiveValueKind,
    pub span: TextSpan,
    pub value_span: TextSpan,
    pub container: DirectiveContainer,
}
