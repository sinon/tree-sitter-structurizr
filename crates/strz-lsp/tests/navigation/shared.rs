use std::{
    fs,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use crate::support::{
    AnnotatedSource, annotated_source, read_workspace_file, workspace_fixture_path,
};

pub const DIRECT_REFERENCES_SOURCE: &str =
    include_str!("../fixtures/relationships/named-relationships-ok.dsl");
pub const DIRECT_REFERENCES_CURSOR_SOURCE: &str =
    include_str!("../fixtures/cursor/relationships/named-relationships-ok.dsl");
pub const SELECTOR_THIS_CURSOR_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "System" {
            <CURSOR:api-declaration>api = container "API"
            worker = container "Worker"

            !element api {
                worker -> <CURSOR:this-reference>this "Targets selector"
            }
        }
    }
}
"#;
pub const THIS_SOURCE_CURSOR_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "System" {
            db = container "DB"
            api = container "API" {
                <CURSOR:repo-declaration>repo = component "Repository" {
                    <CURSOR:this-source>this -> db
                }
            }
        }
    }
}
"#;
pub const ARCHETYPE_THIS_CURSOR_SOURCE: &str = r#"workspace {
    model {
        archetypes {
            application = container
            springBootApplication = application
            repository = component
        }

        x = softwareSystem "X" {
            db = container "DB"
            api = springBootApplication "Customer API" {
                customerController = component "Customer Controller"
                <CURSOR:repo-declaration>customerRepository = repository "Customer Repository" {
                    customerController -> this
                    <CURSOR:this-source>this -> db
                }
            }
        }
    }
}
"#;

pub fn copied_workspace_fixture(name: &str) -> TempDir {
    let source_root = workspace_fixture_path(name);
    let temp_dir = tempfile::Builder::new()
        .prefix(name)
        .tempdir()
        .expect("temp fixture workspace should create");
    copy_workspace_fixture_dir(&source_root, temp_dir.path());
    temp_dir
}

fn copy_workspace_fixture_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap_or_else(|error| {
        panic!(
            "failed to create temp fixture directory `{}`: {error}",
            destination.display()
        )
    });

    for entry in fs::read_dir(source).unwrap_or_else(|error| {
        panic!(
            "failed to read workspace fixture directory `{}`: {error}",
            source.display()
        )
    }) {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to read entry in workspace fixture directory `{}`: {error}",
                source.display()
            )
        });
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().unwrap_or_else(|error| {
            panic!(
                "failed to read file type for workspace fixture entry `{}`: {error}",
                source_path.display()
            )
        });

        if file_type.is_dir() {
            copy_workspace_fixture_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy workspace fixture file `{}` to `{}`: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
        }
    }
}

// Cursor fixtures live under a dedicated test-only tree so the authored marker
// sources stay readable without changing workspace discovery.
pub fn read_annotated_cursor_workspace_fixture(relative_path: &str) -> AnnotatedSource {
    annotated_source(&read_workspace_file(&cursor_workspace_fixture_path(
        relative_path,
    )))
}

fn cursor_workspace_fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cursor/workspaces")
        .join(relative_path)
}
