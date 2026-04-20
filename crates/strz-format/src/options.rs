// =============================================================================
// Formatter policy model
// =============================================================================
//
// The formatter deliberately keeps its policy surface small and code-owned. The
// CLI can later choose how much of this to expose, but the reusable formatter
// core should not bake argument parsing or editor settings into its API.

/// Reusable formatter policy for Structurizr documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOptions {
    indentation: Indentation,
    line_width: LineWidthPolicy,
    spacing: SpacingPolicy,
    syntax_errors: SyntaxErrorPolicy,
}

impl FormatOptions {
    /// Creates a formatter policy from explicit component policies.
    #[must_use]
    pub const fn new(
        indentation: Indentation,
        line_width: LineWidthPolicy,
        spacing: SpacingPolicy,
        syntax_errors: SyntaxErrorPolicy,
    ) -> Self {
        Self {
            indentation,
            line_width,
            spacing,
            syntax_errors,
        }
    }

    /// Returns the indentation policy.
    #[must_use]
    pub const fn indentation(&self) -> Indentation {
        self.indentation
    }

    /// Returns the line-width policy.
    #[must_use]
    pub const fn line_width(&self) -> LineWidthPolicy {
        self.line_width
    }

    /// Returns the vertical-spacing policy.
    #[must_use]
    pub const fn spacing(&self) -> SpacingPolicy {
        self.spacing
    }

    /// Returns the syntax-error handling policy.
    #[must_use]
    pub const fn syntax_errors(&self) -> SyntaxErrorPolicy {
        self.syntax_errors
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self::new(
            Indentation::default(),
            LineWidthPolicy::default(),
            SpacingPolicy::default(),
            SyntaxErrorPolicy::default(),
        )
    }
}

/// Spaces-only indentation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indentation {
    width: u8,
}

impl Indentation {
    /// Creates a spaces-only indentation policy with the given width.
    #[must_use]
    pub const fn spaces(width: u8) -> Self {
        Self { width }
    }

    /// Returns the indentation width in spaces.
    #[must_use]
    pub const fn width(self) -> u8 {
        self.width
    }
}

impl Default for Indentation {
    fn default() -> Self {
        Self::spaces(4)
    }
}

/// Soft line-width policy for ordinary DSL lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineWidthPolicy {
    target: usize,
    comments: CommentFormatPolicy,
    overflow: OverflowPolicy,
}

impl LineWidthPolicy {
    /// Creates a soft line-width policy from explicit sub-policies.
    #[must_use]
    pub const fn new(
        target: usize,
        comments: CommentFormatPolicy,
        overflow: OverflowPolicy,
    ) -> Self {
        Self {
            target,
            comments,
            overflow,
        }
    }

    /// Returns the soft target width for ordinary DSL lines.
    #[must_use]
    pub const fn target(self) -> usize {
        self.target
    }

    /// Returns how comments participate in width handling.
    #[must_use]
    pub const fn comments(self) -> CommentFormatPolicy {
        self.comments
    }

    /// Returns the overflow policy for lines that cannot reasonably be wrapped.
    #[must_use]
    pub const fn overflow(self) -> OverflowPolicy {
        self.overflow
    }
}

impl Default for LineWidthPolicy {
    fn default() -> Self {
        Self::new(
            110,
            CommentFormatPolicy::default(),
            OverflowPolicy::default(),
        )
    }
}

/// Comment formatting policy under the soft line-width target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommentFormatPolicy {
    /// Preserve comment text verbatim and treat width as advisory only.
    #[default]
    Preserve,
}

/// Overflow handling for lines that cannot be cleanly wrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowPolicy {
    /// Treat the width target as best-effort and allow some long lines to remain.
    #[default]
    BestEffort,
}

/// Vertical-spacing policy for block separation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpacingPolicy {
    top_level_gap: u8,
    sibling_block_gap: u8,
}

impl SpacingPolicy {
    /// Creates a spacing policy from explicit blank-line counts.
    #[must_use]
    pub const fn new(top_level_gap: u8, sibling_block_gap: u8) -> Self {
        Self {
            top_level_gap,
            sibling_block_gap,
        }
    }

    /// Returns the blank-line gap between top-level sections.
    #[must_use]
    pub const fn top_level_gap(self) -> u8 {
        self.top_level_gap
    }

    /// Returns the blank-line gap between major sibling block families.
    #[must_use]
    pub const fn sibling_block_gap(self) -> u8 {
        self.sibling_block_gap
    }
}

impl Default for SpacingPolicy {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

/// Syntax-error handling policy for formatter entrypoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyntaxErrorPolicy {
    /// Refuse to rewrite documents that contain parse recovery.
    #[default]
    Refuse,
}
