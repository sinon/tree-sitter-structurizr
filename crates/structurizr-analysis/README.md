# structurizr-analysis

`structurizr-analysis` is the transport-agnostic analysis layer for Structurizr
DSL documents.

It sits between the checked-in Tree-sitter grammar in this repository and the
future language server. Its job is to turn source text into stable,
owned, editor-oriented facts without pulling in LSP types, async runtime
concerns, or runtime-style Structurizr semantics.

## Purpose

This crate exists so that semantic extraction has a clear home that is neither:

- the parser crate, which should stay focused on grammar artifacts such as
  `LANGUAGE`, `NODE_TYPES`, and queries
- the future LSP crate, which should stay focused on protocol handling,
  workspace coordination, and editor integration

The main exchange object is `DocumentSnapshot`, an immutable result of parsing
and analyzing one document.

## Current responsibilities

Today the crate owns:

- parser orchestration via `DocumentAnalyzer` and `analyze_document`
- immutable document inputs and outputs:
  `DocumentInput`, `DocumentId`, `DocumentLocation`, and `DocumentSnapshot`
- transport-agnostic span types: `TextSpan` and `TextPoint`
- syntax diagnostics derived from Tree-sitter error and missing nodes
- raw `!include` facts with source ranges and container context
- raw `!identifiers` facts with declared mode and source ranges
- bounded-MVP symbol extraction for core Structurizr declarations
- bounded-MVP reference extraction for obvious single-document reference sites

The public API intentionally exposes owned facts rather than borrowed
Tree-sitter nodes so snapshots are easy to store, test, and pass across layers.

## What this crate does not own

This crate should not become:

- the grammar crate
- an LSP transport crate
- a Structurizr runtime or full semantic validator

In practice that means it does not own:

- grammar generation, `LANGUAGE`, `NODE_TYPES`, or query packaging
- `lsp-types`, `tower-lsp-server`, editor glue, or async orchestration
- include resolution, workspace graph construction, or runtime-style model
  validation

Multi-file and workspace-aware analysis will layer on later. The current
`WorkspaceFacts` type is only a placeholder for that future work.

## Crate layout

The crate is organized around a small public surface and private tree-walking
extractors:

- `src/parse.rs` - parser setup and analysis entrypoints
- `src/snapshot.rs` - immutable document input and snapshot types
- `src/span.rs` - owned byte and point span types
- `src/diagnostics.rs` - syntax-diagnostic facts
- `src/includes.rs` - raw directive facts for `!include`
- `src/symbols.rs` - symbol, reference, and `!identifiers` facts
- `src/workspace.rs` - placeholder workspace-level types
- `src/extract/` - private Tree-sitter walks that populate the public facts

## Typical usage

For one-off analysis, call `analyze_document`:

```rust
use structurizr_analysis::{DocumentInput, analyze_document};

let source = r#"
workspace {
  model {
    user = person "User"
  }
}
"#;

let snapshot = analyze_document(DocumentInput::new("workspace.dsl", source));

assert_eq!(snapshot.id().as_str(), "workspace.dsl");
println!("symbols: {}", snapshot.symbols().len());
println!("diagnostics: {}", snapshot.syntax_diagnostics().len());
```

If you are analyzing many documents in one process, reuse `DocumentAnalyzer` so
parser setup stays in one place.

## Testing

This crate follows the repository's existing fixture-and-snapshot testing style.

- integration tests live in `crates/structurizr-analysis/tests/`
- shared DSL inputs stay in the repo-root fixture tree under
  `tests/fixtures/lsp/`
- snapshots assert higher-level analysis outputs rather than raw parse trees

The main snapshot test currently exercises:

- syntax diagnostics
- `!include` facts
- `!identifiers` facts
- symbols
- references

Useful commands from the repository root:

```sh
just test-analysis
just test-analysis-fast
cargo test -p structurizr-analysis
```

## Design direction

This crate is meant to stay synchronous, owned, and transport-agnostic for as
long as possible.

That keeps the layering clean:

- `tree-sitter-structurizr` owns syntax
- `structurizr-analysis` owns extracted document facts
- the future LSP crate will own protocol and workspace orchestration

As the editor tooling grows, this crate is the intended home for later
workspace-aware indexing and include-resolution logic, but only after the
single-document analysis surface is stable.
