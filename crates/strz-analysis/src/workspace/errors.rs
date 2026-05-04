// Fatal workspace-load failures and the structured error surface returned when
// discovery cannot assemble complete workspace facts.

// Fatal workspace-load failures and the structured error surface returned when
// discovery cannot assemble complete workspace facts.

/// Fatal failures that can prevent workspace facts from being assembled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceLoadFailureKind {
    /// A requested workspace root could not be normalized or opened.
    WorkspaceRoot,
    /// A workspace root could not be traversed.
    WorkspaceScan,
    /// A discovered file-backed document could not be read.
    DocumentRead,
    /// A `workspace extends` target could not be resolved or loaded.
    WorkspaceBase,
    /// A `workspace extends` chain loops back to an active document.
    WorkspaceBaseCycle,
    /// A local `!include` target could not be loaded after it resolved.
    IncludeLoad,
}

/// Source location for a fatal load failure when the loader can identify one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkspaceLoadFailureAnchor {
    document: DocumentId,
    span: TextSpan,
    value_span: Option<TextSpan>,
    target_text: Option<String>,
}

impl WorkspaceLoadFailureAnchor {
    /// Returns the document that owns the failing directive.
    #[must_use]
    pub const fn document(&self) -> &DocumentId {
        &self.document
    }

    /// Returns the primary source span to highlight.
    #[must_use]
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    /// Returns the narrower value span when the failure came from a directive value.
    #[must_use]
    pub const fn value_span(&self) -> Option<TextSpan> {
        self.value_span
    }

    /// Returns the user-facing target text associated with the failing directive.
    #[must_use]
    pub fn target_text(&self) -> Option<&str> {
        self.target_text.as_deref()
    }
}

/// One structured fatal workspace-load failure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkspaceLoadFailure {
    kind: WorkspaceLoadFailureKind,
    message: String,
    path: Option<PathBuf>,
    anchor: Option<WorkspaceLoadFailureAnchor>,
}

impl WorkspaceLoadFailure {
    fn unanchored(
        kind: WorkspaceLoadFailureKind,
        message: impl Into<String>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            path,
            anchor: None,
        }
    }

    fn anchored(
        kind: WorkspaceLoadFailureKind,
        document: &DocumentId,
        span: TextSpan,
        value_span: Option<TextSpan>,
        target_text: Option<String>,
        path: Option<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            path,
            anchor: Some(WorkspaceLoadFailureAnchor {
                document: document.clone(),
                span,
                value_span,
                target_text,
            }),
        }
    }

    fn workspace_root(path: &Path, error: &io::Error) -> Self {
        Self::unanchored(
            WorkspaceLoadFailureKind::WorkspaceRoot,
            format!("failed to load workspace root {}: {error}", path.display()),
            Some(path.to_path_buf()),
        )
    }

    fn workspace_scan(root: &Path, error: &io::Error) -> Self {
        Self::unanchored(
            WorkspaceLoadFailureKind::WorkspaceScan,
            format!("failed to scan workspace root {}: {error}", root.display()),
            Some(root.to_path_buf()),
        )
    }

    fn document_read(path: &Path, error: &io::Error) -> Self {
        Self::unanchored(
            WorkspaceLoadFailureKind::DocumentRead,
            format!(
                "failed to read workspace document {}: {error}",
                path.display()
            ),
            Some(path.to_path_buf()),
        )
    }

    fn workspace_base(
        document: &DocumentId,
        directive: &WorkspaceBaseDirective,
        target_text: &str,
        path: Option<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self::anchored(
            WorkspaceLoadFailureKind::WorkspaceBase,
            document,
            directive.span,
            Some(directive.value_span),
            Some(target_text.to_owned()),
            path,
            message,
        )
    }

    fn workspace_base_cycle(
        document: &DocumentId,
        directive: &WorkspaceBaseDirective,
        target_text: &str,
        path: PathBuf,
    ) -> Self {
        Self::anchored(
            WorkspaceLoadFailureKind::WorkspaceBaseCycle,
            document,
            directive.span,
            Some(directive.value_span),
            Some(target_text.to_owned()),
            Some(path),
            format!("workspace extends cycle detected while following: {target_text}"),
        )
    }

    fn include_load(
        document: &DocumentId,
        span: TextSpan,
        value_span: TextSpan,
        target_text: &str,
        path: Option<PathBuf>,
        error: &io::Error,
    ) -> Self {
        Self::anchored(
            WorkspaceLoadFailureKind::IncludeLoad,
            document,
            span,
            Some(value_span),
            Some(target_text.to_owned()),
            path,
            format!("failed to load include {target_text}: {error}"),
        )
    }

    /// Returns the typed failure category.
    #[must_use]
    pub const fn kind(&self) -> WorkspaceLoadFailureKind {
        self.kind
    }

    /// Returns the user-facing failure explanation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the relevant filesystem path when one is known.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns source anchor metadata when the failure belongs to a directive.
    #[must_use]
    pub const fn anchor(&self) -> Option<&WorkspaceLoadFailureAnchor> {
        self.anchor.as_ref()
    }

    /// Returns whether this failure can be rendered as a source diagnostic.
    #[must_use]
    pub const fn is_anchored(&self) -> bool {
        self.anchor.is_some()
    }

    /// Converts an anchored load failure into the normal ruled diagnostic stream.
    #[must_use]
    pub fn diagnostic(&self) -> Option<RuledDiagnostic> {
        let anchor = self.anchor()?;

        Some(RuledDiagnostic::workspace_load_failure(
            anchor.document(),
            self.message(),
            anchor.span(),
            anchor.value_span(),
            anchor.target_text(),
        ))
    }
}

/// Structured fatal result for workspace loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLoadError {
    failures: Vec<WorkspaceLoadFailure>,
}

impl WorkspaceLoadError {
    fn single(failure: WorkspaceLoadFailure) -> Self {
        Self {
            failures: vec![failure],
        }
    }

    /// Returns the structured failures that caused the load to abort.
    #[must_use]
    pub fn failures(&self) -> &[WorkspaceLoadFailure] {
        &self.failures
    }

    /// Consumes the error and returns its structured failures.
    #[must_use]
    pub fn into_failures(self) -> Vec<WorkspaceLoadFailure> {
        self.failures
    }
}

impl fmt::Display for WorkspaceLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.failures.as_slice() {
            [] => formatter.write_str("workspace load failed"),
            [failure] => formatter.write_str(failure.message()),
            failures => {
                write!(
                    formatter,
                    "workspace load failed with {} fatal failures",
                    failures.len()
                )?;
                for failure in failures {
                    write!(formatter, "\n- {}", failure.message())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for WorkspaceLoadError {}

type WorkspaceLoadResult<T> = Result<T, WorkspaceLoadError>;
