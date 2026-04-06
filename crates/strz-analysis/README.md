# strz-analysis

`strz-analysis` is the transport-agnostic analysis layer for Structurizr
DSL documents.

It sits between the checked-in Tree-sitter grammar in this repository and the
tools that consume semantic facts, such as the `strz check` and `strz lsp`.
Its job is to turn source text into stable, owned, editor-oriented facts
without pulling in LSP types, async runtime concerns, or runtime-style
Structurizr semantics.

## Purpose

This crate exists so that semantic extraction has a clear home that is neither:

- the parser crate, which should stay focused on grammar artifacts such as
  `LANGUAGE`, `NODE_TYPES`, and queries
- the LSP crate, which should stay focused on protocol handling,
  workspace coordination, and editor integration
- the `strz` CLI, which should stay focused on rendering and
  command-line UX rather than owning semantic extraction

The main exchange object is `DocumentSnapshot`, an immutable result of parsing
and analyzing one document.

## Current responsibilities

Today the crate owns:

- parser orchestration via `DocumentAnalyzer`
- immutable document inputs and outputs:
  `DocumentInput`, `DocumentId`, `DocumentLocation`, and `DocumentSnapshot`
- transport-agnostic span types: `TextSpan` and `TextPoint`
- syntax diagnostics derived from Tree-sitter error and missing nodes
- raw `!include` facts with source ranges and container context
- raw `!identifiers` facts with declared mode and source ranges
- bounded-MVP symbol extraction for core Structurizr declarations
- bounded-MVP reference extraction for obvious single-document reference sites
- initial workspace discovery for `.dsl` roots plus explicit include-following
- normalized include target facts for local files, local directories, and remote
  URLs
- file-level include diagnostics for missing targets, subtree escapes, cycles, and
  unresolved remote includes

The public API intentionally exposes owned facts rather than borrowed
Tree-sitter nodes so snapshots are easy to store, test, and pass across layers.

## Crate layout

The crate is organized around a small public surface and private tree-walking
extractors:

- [`src/parse.rs`](src/parse.rs) - parser setup and analysis entrypoints
- [`src/snapshot.rs`](src/snapshot.rs) - immutable document input and snapshot types
- [`src/span.rs`](src/span.rs) - owned byte and point span types
- [`src/diagnostics.rs`](src/diagnostics.rs) - syntax-diagnostic facts
- [`src/includes.rs`](src/includes.rs) - raw directive facts for `!include`
- [`src/symbols.rs`](src/symbols.rs) - symbol, reference, and `!identifiers` facts
- [`src/workspace.rs`](src/workspace.rs) - workspace discovery and include-following
- [`src/extract/`](src/extract/) - private Tree-sitter walks that populate the public facts

## Typical usage

Create a `DocumentAnalyzer` and analyze one or more documents through that
stateful entrypoint:

```rust
use strz_analysis::{DocumentAnalyzer, DocumentInput};

let source = r#"
workspace {
  model {
    user = person "User"
  }
}
"#;

let mut analyzer = DocumentAnalyzer::new();
let snapshot = analyzer.analyze(DocumentInput::new("workspace.dsl", source));

assert_eq!(snapshot.id().as_str(), "workspace.dsl");
println!("symbols: {}", snapshot.symbols().len());
println!("diagnostics: {}", snapshot.syntax_diagnostics().len());
```

If you are analyzing many documents or repeated edits in one process, keep one
`DocumentAnalyzer` alive so its parser setup and incremental cache stay in one
place.

## Testing

This crate follows the repository's existing fixture-and-snapshot testing style.

- integration tests live in [`crates/strz-analysis/tests/`](tests/)
- LSP-specific single-document DSL inputs live under
  [`crates/strz-lsp/tests/fixtures/`](../strz-lsp/tests/fixtures/)
- snapshots assert higher-level analysis outputs rather than raw parse trees

The main snapshot test currently exercises:

- syntax diagnostics
- `!include` facts
- `!identifiers` facts
- symbols
- references

Workspace discovery has its own integration tests under
[`crates/strz-analysis/tests/workspace_discovery.rs`](tests/workspace_discovery.rs), using shared
workspace fixtures under [`tests/lsp/workspaces/`](../../tests/lsp/workspaces/).

Useful commands from the repository root:

```sh
just test-analysis
just test-analysis-fast
cargo test -p strz-analysis
```
