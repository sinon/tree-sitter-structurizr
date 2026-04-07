//! Declarative metadata for diagnostic rules.

use std::collections::BTreeMap;

/// Default emission level for one registered diagnostic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Emit the rule as a warning.
    Warn,
    /// Emit the rule as an error.
    Error,
}

impl Level {
    /// Returns the stable string form used in docs and debugging.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Declarative metadata for one diagnostic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Stable diagnostic code used by CLI and LSP consumers.
    pub code: &'static str,
    /// One-line summary of what the rule reports.
    pub summary: &'static str,
    /// Markdown-ish rule documentation carried from the declaration site.
    pub raw_documentation: &'static str,
    /// Default emission level when the analysis layer reports this rule.
    pub default_level: Level,
    /// Source file where the rule was declared.
    pub file: &'static str,
    /// One-based line number where the rule was declared.
    pub line: u32,
}

pub const fn rule_metadata_defaults() -> RuleMetadata {
    RuleMetadata {
        code: "",
        summary: "",
        raw_documentation: "",
        default_level: Level::Error,
        file: "",
        line: 1,
    }
}

macro_rules! declare_rule {
    (
        $(#[doc = $doc:literal])+
        $vis:vis static $name:ident = {
            code: $code:literal,
            summary: $summary:literal,
            default_level: $default_level:expr $(,)?
        };
    ) => {
        $( #[doc = $doc] )+
        #[expect(clippy::needless_update)]
        $vis static $name: $crate::rule::RuleMetadata = $crate::rule::RuleMetadata {
            code: $code,
            summary: $summary,
            raw_documentation: concat!($($doc, '\n',)+),
            default_level: $default_level,
            file: file!(),
            line: line!(),
            ..$crate::rule::rule_metadata_defaults()
        };
    };
}

pub(crate) use declare_rule;

/// Builder used to assemble one stable registry of diagnostic rules.
#[derive(Debug, Default)]
pub struct RuleRegistryBuilder {
    rules: Vec<&'static RuleMetadata>,
    by_code: BTreeMap<&'static str, &'static RuleMetadata>,
}

impl RuleRegistryBuilder {
    /// Registers one diagnostic rule definition.
    ///
    /// # Panics
    ///
    /// Panics when another registered rule already uses the same stable
    /// diagnostic code.
    pub fn register(&mut self, rule: &'static RuleMetadata) {
        if let Some(existing) = self.by_code.insert(rule.code, rule) {
            panic!(
                "BUG: duplicate diagnostic rule code `{}` declared in {}:{} and {}:{}",
                rule.code, existing.file, existing.line, rule.file, rule.line,
            );
        }
        self.rules.push(rule);
    }

    /// Finalizes the registry into a stable lookup structure.
    #[must_use]
    pub fn build(mut self) -> RuleRegistry {
        self.rules.sort_by(|left, right| left.code.cmp(right.code));
        RuleRegistry {
            rules: self.rules,
            by_code: self.by_code,
        }
    }
}

/// Stable registry of known diagnostic rules.
#[derive(Debug)]
pub struct RuleRegistry {
    rules: Vec<&'static RuleMetadata>,
    by_code: BTreeMap<&'static str, &'static RuleMetadata>,
}

impl RuleRegistry {
    /// Returns all registered diagnostic rules in deterministic order.
    #[must_use]
    pub fn all(&self) -> &[&'static RuleMetadata] {
        &self.rules
    }

    /// Looks up one diagnostic rule by its stable diagnostic code.
    #[must_use]
    pub fn get(&self, code: &str) -> Option<&'static RuleMetadata> {
        self.by_code.get(code).copied()
    }
}
