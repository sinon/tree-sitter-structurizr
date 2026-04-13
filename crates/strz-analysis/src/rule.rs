//! Declarative metadata for diagnostic rules.

use std::{collections::BTreeMap, fmt, hash::Hash};

/// Severity used both for declared rule defaults and emitted diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Emit or present the rule as a warning.
    Warning,
    /// Emit the rule as an error.
    Error,
}

impl DiagnosticSeverity {
    /// Returns the stable string form shared by docs, JSON, and text output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Declarative metadata for one diagnostic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Stable diagnostic code used by CLI and LSP consumers.
    ///
    /// This is derived from the rule's broad `source` family plus its local
    /// declared name so those two surfaces cannot drift apart.
    pub code: &'static str,
    /// Broad analysis stage name used by current CLI/LSP renderers.
    pub source: &'static str,
    /// One-line summary of what the rule reports.
    pub summary: &'static str,
    /// Markdown-ish rule documentation carried from the declaration site.
    pub raw_documentation: &'static str,
    /// Default severity when the analysis layer reports this rule.
    pub default_severity: DiagnosticSeverity,
    /// Source file where the rule was declared.
    pub file: &'static str,
    /// One-based line number where the rule was declared.
    pub line: u32,
}

impl RuleMetadata {
    /// Returns a cheap copyable handle for this declared rule.
    #[must_use]
    pub const fn id(&'static self) -> RuleId {
        RuleId::of(self)
    }

    /// Returns the stable diagnostic code for this rule.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the broad analysis-stage label for this rule.
    #[must_use]
    pub const fn source(&self) -> &'static str {
        self.source
    }

    /// Returns the one-line summary for this rule.
    #[must_use]
    pub const fn summary(&self) -> &'static str {
        self.summary
    }

    /// Returns the declaration-site documentation for this rule.
    #[must_use]
    pub const fn documentation(&self) -> &'static str {
        self.raw_documentation
    }

    /// Returns the default severity for this rule.
    #[must_use]
    pub const fn default_severity(&self) -> DiagnosticSeverity {
        self.default_severity
    }

    /// Returns the severity for this rule.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.default_severity
    }

    /// Returns the source file where this rule was declared.
    #[must_use]
    pub const fn file(&self) -> &'static str {
        self.file
    }

    /// Returns the one-based line number where this rule was declared.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }
}

pub const fn rule_metadata_defaults() -> RuleMetadata {
    RuleMetadata {
        code: "",
        source: "",
        summary: "",
        raw_documentation: "",
        default_severity: DiagnosticSeverity::Error,
        file: "",
        line: 1,
    }
}

macro_rules! declare_rule {
    (
        $(#[doc = $doc:literal])+
        $vis:vis static $name:ident = {
            name: $rule_name:literal,
            source: $source:literal,
            summary: $summary:literal,
            default_severity: $default_severity:expr $(,)?
        };
    ) => {
        $( #[doc = $doc] )+
        #[expect(clippy::needless_update)]
        $vis static $name: $crate::rule::RuleMetadata = $crate::rule::RuleMetadata {
            code: concat!($source, ".", $rule_name),
            source: $source,
            summary: $summary,
            raw_documentation: concat!($($doc, '\n',)+),
            default_severity: $default_severity,
            file: file!(),
            line: line!(),
            ..$crate::rule::rule_metadata_defaults()
        };
    };
}

pub(crate) use declare_rule;

/// Cheap copyable identity for one declared rule.
#[derive(Clone, Copy)]
pub struct RuleId {
    definition: &'static RuleMetadata,
}

impl RuleId {
    /// Creates a rule identifier from one declared rule.
    #[must_use]
    pub const fn of(definition: &'static RuleMetadata) -> Self {
        Self { definition }
    }

    /// Returns the underlying declarative metadata.
    #[must_use]
    pub const fn metadata(self) -> &'static RuleMetadata {
        self.definition
    }

    /// Returns the stable diagnostic code for this rule.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.definition.code()
    }

    /// Returns the current broad source-family label for this rule.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.definition.source()
    }

    /// Returns the transport-facing severity implied by this rule.
    #[must_use]
    pub const fn severity(self) -> DiagnosticSeverity {
        self.definition.severity()
    }
}

impl std::ops::Deref for RuleId {
    type Target = RuleMetadata;

    fn deref(&self) -> &Self::Target {
        self.definition
    }
}

impl fmt::Debug for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl PartialEq for RuleId {
    fn eq(&self, other: &Self) -> bool {
        self.code() == other.code()
    }
}

impl Eq for RuleId {}

impl PartialOrd for RuleId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RuleId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.code().cmp(other.code())
    }
}

impl Hash for RuleId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.code().hash(state);
    }
}

/// Builder used to assemble one stable registry of diagnostic rules.
#[derive(Debug, Default)]
pub struct RuleRegistryBuilder {
    rules: Vec<RuleId>,
    by_code: BTreeMap<&'static str, RuleId>,
}

impl RuleRegistryBuilder {
    /// Registers one diagnostic rule definition.
    ///
    /// # Panics
    ///
    /// Panics when another registered rule already uses the same stable
    /// diagnostic code.
    pub fn register(&mut self, rule: &'static RuleMetadata) {
        let rule_id = rule.id();
        if let Some(existing) = self.by_code.insert(rule.code(), rule_id) {
            panic!(
                "BUG: duplicate diagnostic rule code `{}` declared in {}:{} and {}:{}",
                rule.code(),
                existing.file(),
                existing.line(),
                rule.file,
                rule.line,
            );
        }
        self.rules.push(rule_id);
    }

    /// Finalizes the registry into a stable lookup structure.
    #[must_use]
    pub fn build(mut self) -> RuleRegistry {
        self.rules
            .sort_by(|left, right| left.code().cmp(right.code()));
        RuleRegistry {
            rules: self.rules,
            by_code: self.by_code,
        }
    }
}

/// Stable registry of known diagnostic rules.
#[derive(Debug)]
pub struct RuleRegistry {
    rules: Vec<RuleId>,
    by_code: BTreeMap<&'static str, RuleId>,
}

impl RuleRegistry {
    /// Returns all registered diagnostic rules in deterministic order.
    #[must_use]
    pub fn all(&self) -> &[RuleId] {
        &self.rules
    }

    /// Looks up one diagnostic rule by its stable diagnostic code.
    #[must_use]
    pub fn get(&self, code: &str) -> Option<RuleId> {
        self.by_code.get(code).copied()
    }
}
