use std::{
    path::{Path, PathBuf},
    process::Command,
};

use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};

const BIN_NAME: &str = "strz";

#[test]
fn root_help_lists_server_subcommand() {
    let output = command()
        .arg("--help")
        .output()
        .expect("help command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "help should exit successfully");
    assert!(
        stdout.contains("Usage: strz [OPTIONS] <COMMAND>"),
        "root help should advertise the strz binary name"
    );
    assert!(
        stdout.contains("server  Run the Structurizr LSP server over stdio"),
        "root help should list the server subcommand"
    );
    assert!(
        stdout.contains("check   Check one or more files or directories"),
        "root help should preserve the check subcommand"
    );
}

#[test]
fn check_text_reports_missing_include() {
    assert_cmd_snapshot!(
        command()
            .arg("check")
            .arg("tests/lsp/workspaces/missing-include"),
        @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    tests/lsp/workspaces/missing-include/workspace.dsl:2:5: error[include.missing-local-target] included path does not exist: missing/model.dsl

    ----- stderr -----
    "###
    );
}

#[test]
fn check_json_reports_missing_include() {
    assert_cmd_snapshot!(
        command()
            .arg("--output-format")
            .arg("json")
            .arg("check")
            .arg("tests/lsp/workspaces/missing-include"),
        @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    {
      "summary": {
        "documents_checked": 1,
        "diagnostics": 1,
        "errors": 1,
        "warnings": 0
      },
      "diagnostics": [
        {
          "path": "tests/lsp/workspaces/missing-include/workspace.dsl",
          "severity": "error",
          "code": "include.missing-local-target",
          "source": "include",
          "message": "included path does not exist: missing/model.dsl",
          "span": {
            "start_byte": 16,
            "end_byte": 44,
            "start": {
              "line": 2,
              "column": 5
            },
            "end": {
              "line": 2,
              "column": 33
            }
          }
        }
      ]
    }

    ----- stderr -----
    "###
    );
}

#[test]
fn check_json_big_bank_plc_reports_current_golden_record_diagnostics() {
    assert_cmd_snapshot!(
        command()
            .arg("--output-format")
            .arg("json")
            .arg("check")
            .arg("tests/lsp/workspaces/big-bank-plc")
    );
}

#[test]
fn dump_document_text_reports_symbols_and_references() {
    assert_cmd_snapshot!(
        command()
            .arg("dump")
            .arg("document")
            .arg("tests/fixtures/lsp/identifiers/direct-references-ok.dsl"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    document:
    path: tests/fixtures/lsp/identifiers/direct-references-ok.dsl
    workspace_entry: true
    syntax diagnostics:
    - none
    include directives:
    - none
    identifier modes:
    - none
    symbols:
    - #0 kind=person display_name=User binding_name=user parent=None at 3:9
    - #1 kind=software_system display_name=System binding_name=system parent=None at 4:9
    - #2 kind=container display_name=API binding_name=api parent=Some(1) at 5:13
    - #3 kind=component display_name=Worker binding_name=worker parent=Some(2) at 6:17
    references:
    - kind=relationship_source raw_text=user target_hint=element containing_symbol=None at 10:9
    - kind=relationship_destination raw_text=system target_hint=element containing_symbol=None at 10:17
    - kind=relationship_source raw_text=user target_hint=element containing_symbol=None at 11:9
    - kind=relationship_destination raw_text=api target_hint=element containing_symbol=None at 11:17
    - kind=relationship_source raw_text=user target_hint=element containing_symbol=None at 12:9
    - kind=relationship_destination raw_text=worker target_hint=element containing_symbol=None at 12:17
    - kind=view_scope raw_text=system target_hint=element containing_symbol=None at 16:23
    - kind=view_include raw_text=user target_hint=element_or_relationship containing_symbol=None at 17:21
    - kind=view_scope raw_text=system target_hint=element containing_symbol=None at 20:19
    - kind=view_include raw_text=api target_hint=element_or_relationship containing_symbol=None at 21:21
    - kind=view_scope raw_text=api target_hint=element containing_symbol=None at 24:19
    - kind=view_include raw_text=worker target_hint=element_or_relationship containing_symbol=None at 25:21

    ----- stderr -----
    "###
    );
}

#[test]
fn dump_workspace_json_reports_discovery_and_include_expansion() {
    assert_cmd_snapshot!(
        command()
            .arg("--output-format")
            .arg("json")
            .arg("dump")
            .arg("workspace")
            .arg("tests/lsp/workspaces/directory-include"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    {
      "roots": [
        "tests/lsp/workspaces/directory-include"
      ],
      "entry_documents": [
        "tests/lsp/workspaces/directory-include/workspace.dsl"
      ],
      "documents": [
        {
          "path": "tests/lsp/workspaces/directory-include/fragments/10-model.dsl",
          "kind": "fragment",
          "discovered_by_scan": true,
          "syntax_diagnostics": [],
          "include_directive_count": 0,
          "symbol_count": 1,
          "reference_count": 0
        },
        {
          "path": "tests/lsp/workspaces/directory-include/fragments/20-views.dsl",
          "kind": "fragment",
          "discovered_by_scan": true,
          "syntax_diagnostics": [],
          "include_directive_count": 0,
          "symbol_count": 0,
          "reference_count": 0
        },
        {
          "path": "tests/lsp/workspaces/directory-include/fragments/nested/30-system.dsl",
          "kind": "fragment",
          "discovered_by_scan": true,
          "syntax_diagnostics": [],
          "include_directive_count": 0,
          "symbol_count": 1,
          "reference_count": 0
        },
        {
          "path": "tests/lsp/workspaces/directory-include/workspace.dsl",
          "kind": "entry",
          "discovered_by_scan": true,
          "syntax_diagnostics": [],
          "include_directive_count": 1,
          "symbol_count": 0,
          "reference_count": 0
        }
      ],
      "includes": [
        {
          "document": "tests/lsp/workspaces/directory-include/workspace.dsl",
          "target_kind": "local_directory",
          "target_text": "fragments",
          "raw_value": "\"fragments\"",
          "span": {
            "start_byte": 16,
            "end_byte": 36,
            "start": {
              "line": 2,
              "column": 5
            },
            "end": {
              "line": 2,
              "column": 25
            }
          },
          "value_span": {
            "start_byte": 25,
            "end_byte": 36,
            "start": {
              "line": 2,
              "column": 14
            },
            "end": {
              "line": 2,
              "column": 25
            }
          },
          "target_location": "tests/lsp/workspaces/directory-include/fragments",
          "discovered_documents": [
            "tests/lsp/workspaces/directory-include/fragments/10-model.dsl",
            "tests/lsp/workspaces/directory-include/fragments/20-views.dsl",
            "tests/lsp/workspaces/directory-include/fragments/nested/30-system.dsl"
          ]
        }
      ],
      "include_diagnostics": []
    }

    ----- stderr -----
    "###
    );
}

#[test]
fn check_counts_explicit_non_dsl_file_roots() {
    assert_cmd_snapshot!(
        command()
            .arg("check")
            .arg("tests/lsp/workspaces/ignored-explicit/ignored/model.inc"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    No diagnostics found in 1 document(s).

    ----- stderr -----
    "###
    );
}

#[test]
fn runtime_errors_honor_color_choice() {
    let always_output = command()
        .arg("--color")
        .arg("always")
        .arg("dump")
        .arg("document")
        .arg("does-not-exist.dsl")
        .output()
        .expect("runtime error command should run");
    assert_eq!(always_output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&always_output.stderr).contains("\u{1b}["),
        "expected ANSI color escapes in stderr when --color=always is set"
    );

    let never_output = command()
        .arg("--color")
        .arg("never")
        .arg("dump")
        .arg("document")
        .arg("does-not-exist.dsl")
        .output()
        .expect("runtime error command should run");
    assert_eq!(never_output.status.code(), Some(2));
    assert!(
        !String::from_utf8_lossy(&never_output.stderr).contains("\u{1b}["),
        "expected plain stderr when --color=never is set"
    );
}

fn command() -> Command {
    let mut command = Command::new(get_cargo_bin(BIN_NAME));
    command.current_dir(repo_root());
    command
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate path should have a repo root ancestor")
        .to_path_buf()
}
