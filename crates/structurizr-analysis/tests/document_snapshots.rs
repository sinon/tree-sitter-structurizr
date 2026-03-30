use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;
use structurizr_analysis::{DocumentInput, DocumentSnapshot, analyze_document};

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[allow(dead_code)]
#[derive(Debug)]
struct SnapshotView {
    has_syntax_errors: bool,
    diagnostics: Vec<structurizr_analysis::SyntaxDiagnostic>,
    include_directives: Vec<structurizr_analysis::IncludeDirective>,
    identifier_modes: Vec<structurizr_analysis::IdentifierModeFact>,
    symbols: Vec<structurizr_analysis::Symbol>,
    references: Vec<structurizr_analysis::Reference>,
}

impl From<&DocumentSnapshot> for SnapshotView {
    fn from(snapshot: &DocumentSnapshot) -> Self {
        Self {
            has_syntax_errors: snapshot.has_syntax_errors(),
            diagnostics: snapshot.syntax_diagnostics().to_vec(),
            include_directives: snapshot.include_directives().to_vec(),
            identifier_modes: snapshot.identifier_modes().to_vec(),
            symbols: snapshot.symbols().to_vec(),
            references: snapshot.references().to_vec(),
        }
    }
}

#[rstest]
fn lsp_fixtures_produce_stable_snapshots(
    #[files("../structurizr-grammar/tests/fixtures/lsp/**/*.dsl")] path: PathBuf,
) {
    let snapshot = analyze_fixture(&path);

    set_snapshot_suffix!("{}", relative_fixture_name(&path));
    insta::assert_debug_snapshot!("document_snapshot", SnapshotView::from(&snapshot));
}

fn analyze_fixture(path: &Path) -> DocumentSnapshot {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display()));

    analyze_document(DocumentInput::new(relative_fixture_name(path), source).with_location(path))
}

fn relative_fixture_name(path: &Path) -> String {
    let fixture_root =
        fs::canonicalize(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../structurizr-grammar/tests/fixtures/lsp"),
        )
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
                "fixture path should live under crates/structurizr-grammar/tests/fixtures/lsp: {}",
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
