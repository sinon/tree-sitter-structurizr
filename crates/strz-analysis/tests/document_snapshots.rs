use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;
use strz_analysis::{DocumentAnalyzer, DocumentInput, DocumentSnapshot};

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
    diagnostics: Vec<strz_analysis::SyntaxDiagnostic>,
    include_directives: Vec<strz_analysis::IncludeDirective>,
    identifier_modes: Vec<strz_analysis::IdentifierModeFact>,
    symbols: Vec<strz_analysis::Symbol>,
    references: Vec<strz_analysis::Reference>,
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
    #[files("../strz-lsp/tests/fixtures/**/*.dsl")] path: PathBuf,
) {
    assert_fixture_snapshot(
        &path,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../strz-lsp/tests/fixtures"),
    );
}

#[rstest]
fn shared_fixtures_produce_stable_snapshots(
    #[files("../../fixtures/deployment/deployment-parent-child-relationship-err.dsl")]
    path: PathBuf,
) {
    assert_fixture_snapshot(
        &path,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures"),
    );
}

fn analyze_fixture(path: &Path, document_name: &str) -> DocumentSnapshot {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display()));

    let mut analyzer = DocumentAnalyzer::new();
    analyzer.analyze(DocumentInput::new(document_name, source).with_location(path))
}

fn assert_fixture_snapshot(path: &Path, fixture_root: &Path) {
    let fixture_name = relative_fixture_name(path, fixture_root);
    let snapshot = analyze_fixture(path, &fixture_name);

    set_snapshot_suffix!("{}", fixture_name);
    insta::assert_debug_snapshot!("document_snapshot", SnapshotView::from(&snapshot));
}

fn relative_fixture_name(path: &Path, fixture_root: &Path) -> String {
    let fixture_root = fs::canonicalize(fixture_root)
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
                "fixture path should live under {}: {}",
                fixture_root.display(),
                path.display(),
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
