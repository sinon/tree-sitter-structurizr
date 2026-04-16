use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;
use strz_analysis::{
    DirectiveContainer, DirectiveValueKind, DocumentAnalyzer, DocumentInput, DocumentSnapshot,
    IdentifierMode, ReferenceKind, ReferenceTargetHint, SymbolKind, WorkspaceSectionKind,
};
use strz_format::Formatter;

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
fn fixtures_format_to_stable_snapshots(#[files("tests/fixtures/**/*.dsl")] path: PathBuf) {
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display()));
    let fixture_name = relative_fixture_name(&path);

    let mut formatter = Formatter::default();
    let first_pass = formatter
        .format_document(
            DocumentInput::new(fixture_name.clone(), source.clone()).with_location(path.as_path()),
        )
        .unwrap_or_else(|error| {
            panic!(
                "fixture `{}` should format cleanly: {error}",
                path.display()
            )
        });
    let formatted = first_pass.formatted().to_owned();
    let second_pass = formatter
        .format_document(
            DocumentInput::new(fixture_name.clone(), formatted.clone())
                .with_location(path.as_path()),
        )
        .unwrap_or_else(|error| {
            panic!(
                "formatted fixture `{}` should remain formatable: {error}",
                path.display()
            )
        });

    set_snapshot_suffix!("{}", fixture_name);
    insta::assert_snapshot!("formatted_document", formatted);

    assert_eq!(
        second_pass.formatted(),
        first_pass.formatted(),
        "formatter should be idempotent for `{}`",
        path.display()
    );
    assert_semantics_preserved(&source, first_pass.formatted(), &path, &fixture_name);
}

fn assert_semantics_preserved(
    before_source: &str,
    after_source: &str,
    path: &Path,
    fixture_name: &str,
) {
    let before = analyze(path, fixture_name, before_source);
    let after = analyze(path, fixture_name, after_source);

    assert_eq!(
        SemanticView::from(&before),
        SemanticView::from(&after),
        "formatter changed analysis facts for `{}`",
        path.display()
    );
}

fn analyze(path: &Path, fixture_name: &str, source: &str) -> DocumentSnapshot {
    let mut analyzer = DocumentAnalyzer::new();
    analyzer
        .analyze(DocumentInput::new(fixture_name.to_owned(), source.to_owned()).with_location(path))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticView {
    has_syntax_errors: bool,
    include_directives: Vec<IncludeDirectiveView>,
    identifier_modes: Vec<IdentifierModeView>,
    symbols: Vec<SymbolView>,
    references: Vec<ReferenceView>,
    workspace_sections: Vec<WorkspaceSectionKind>,
}

impl From<&DocumentSnapshot> for SemanticView {
    fn from(snapshot: &DocumentSnapshot) -> Self {
        Self {
            has_syntax_errors: snapshot.has_syntax_errors(),
            include_directives: snapshot
                .include_directives()
                .iter()
                .map(|directive| IncludeDirectiveView {
                    raw_value: directive.raw_value.clone(),
                    value_kind: directive.value_kind.clone(),
                    container: directive.container.clone(),
                })
                .collect(),
            identifier_modes: snapshot
                .identifier_modes()
                .iter()
                .map(|fact| IdentifierModeView {
                    mode: fact.mode.clone(),
                    raw_value: fact.raw_value.clone(),
                    value_kind: fact.value_kind.clone(),
                    container: fact.container.clone(),
                })
                .collect(),
            symbols: snapshot
                .symbols()
                .iter()
                .map(|symbol| SymbolView {
                    kind: symbol.kind,
                    display_name: symbol.display_name.clone(),
                    binding_name: symbol.binding_name.clone(),
                    description: symbol.description.clone(),
                    technology: symbol.technology.clone(),
                    tags: symbol.tags.clone(),
                    url: symbol.url.clone(),
                    parent: symbol.parent.map(|parent| parent.0),
                    syntax_node_kind: symbol.syntax_node_kind.clone(),
                })
                .collect(),
            references: snapshot
                .references()
                .iter()
                .map(|reference| ReferenceView {
                    kind: reference.kind,
                    raw_text: reference.raw_text.clone(),
                    target_hint: reference.target_hint,
                    container_node_kind: reference.container_node_kind.clone(),
                    containing_symbol: reference.containing_symbol.map(|symbol| symbol.0),
                })
                .collect(),
            workspace_sections: snapshot
                .workspace_sections()
                .iter()
                .map(|section| section.kind)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeDirectiveView {
    raw_value: String,
    value_kind: DirectiveValueKind,
    container: DirectiveContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdentifierModeView {
    mode: IdentifierMode,
    raw_value: String,
    value_kind: DirectiveValueKind,
    container: DirectiveContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolView {
    kind: SymbolKind,
    display_name: String,
    binding_name: Option<String>,
    description: Option<String>,
    technology: Option<String>,
    tags: Vec<String>,
    url: Option<String>,
    parent: Option<usize>,
    syntax_node_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceView {
    kind: ReferenceKind,
    raw_text: String,
    target_hint: ReferenceTargetHint,
    container_node_kind: String,
    containing_symbol: Option<usize>,
}

fn relative_fixture_name(path: &Path) -> String {
    let fixture_root =
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .unwrap_or_else(|error| panic!("failed to canonicalize fixture root: {error}"));
    let fixture_path = fs::canonicalize(path).unwrap_or_else(|error| {
        panic!(
            "failed to canonicalize fixture `{}`: {error}",
            path.display()
        )
    });
    let relative = fixture_path
        .strip_prefix(&fixture_root)
        .unwrap_or_else(|_| {
            panic!(
                "fixture path should live under tests/fixtures: {}",
                path.display()
            )
        })
        .with_extension("");

    relative
        .components()
        .map(|component| fixture_name_component(&component.as_os_str().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("__")
}

fn fixture_name_component(component: &str) -> String {
    component.strip_suffix("-ok").map_or_else(
        || {
            component.strip_suffix("-err").map_or_else(
                || normalize_fixture_component(component),
                |base| format!("{}_err", normalize_fixture_component(base)),
            )
        },
        |base| format!("{}_ok", normalize_fixture_component(base)),
    )
}

fn normalize_fixture_component(component: &str) -> String {
    component.replace(['-', '.'], "_")
}
