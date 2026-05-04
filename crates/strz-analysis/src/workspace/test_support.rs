// Shared test fixtures and assertions for the split workspace module tests.

fn workspace_index_for_root<'a>(
    facts: &'a WorkspaceFacts,
    root_path: &std::path::Path,
) -> &'a WorkspaceIndex {
    let root_document = document_id_from_path(root_path);
    facts
        .workspace_indexes()
        .iter()
        .find(|index| index.root_document() == &root_document)
        .expect("workspace index for root should exist")
}

fn assert_reference_resolves_to(
    index: &WorkspaceIndex,
    document_id: &DocumentId,
    references: &[Reference],
    kind: ReferenceKind,
    raw_text: &str,
    expected_target: SymbolHandle,
) {
    let reference_index = references
        .iter()
        .enumerate()
        .find(|(_, reference)| reference.kind == kind && reference.raw_text == raw_text)
        .map_or_else(
            || {
                let available = references
                    .iter()
                    .map(|reference| format!("{:?} `{}`", reference.kind, reference.raw_text))
                    .collect::<Vec<_>>();
                panic!("expected {kind:?} reference `{raw_text}`, got {available:?}")
            },
            |(index, _)| index,
        );
    let handle = ReferenceHandle::new(document_id.clone(), reference_index);

    assert_eq!(
        index.reference_resolution(&handle),
        Some(&ReferenceResolutionStatus::Resolved(expected_target)),
        "{kind:?} `{raw_text}` should resolve"
    );
}

fn symbol_id_by_binding(symbols: &[Symbol], binding_name: &str) -> SymbolId {
    symbols
        .iter()
        .find(|symbol| symbol.binding_name.as_deref() == Some(binding_name))
        .unwrap_or_else(|| panic!("expected symbol binding `{binding_name}`"))
        .id
}

struct TemporaryWorkspace {
    _root_dir: tempfile::TempDir,
    root: std::path::PathBuf,
    workspace_path: std::path::PathBuf,
}

impl TemporaryWorkspace {
    fn new(workspace_source: &str) -> Self {
        let root_dir = tempfile::tempdir().expect("tempdir should create");
        let root = root_dir
            .path()
            .canonicalize()
            .expect("tempdir path should canonicalize");
        let workspace_path = root.join("workspace.dsl");
        std::fs::write(&workspace_path, workspace_source).expect("workspace source should write");

        Self {
            _root_dir: root_dir,
            root,
            workspace_path,
        }
    }

    fn write_model(&self, source: &str) {
        std::fs::write(self.model_path(), source).expect("model source should write");
    }

    fn write_file(&self, relative_path: &str, source: &str) {
        std::fs::write(self.root.join(relative_path), source).expect("workspace file should write");
    }

    #[allow(clippy::missing_const_for_fn)]
    fn root(&self) -> &std::path::Path {
        &self.root
    }

    #[allow(clippy::missing_const_for_fn)]
    fn workspace_path(&self) -> &std::path::PathBuf {
        &self.workspace_path
    }

    fn model_path(&self) -> std::path::PathBuf {
        self.root.join("model.dsl")
    }
}
