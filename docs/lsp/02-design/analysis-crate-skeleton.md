# Structurizr DSL analysis crate skeleton

> Status: implemented in bounded form.
>
> `structurizr-analysis` now exists in-repo. Read this note as the architectural
> contract for that crate shape and as background for future expansion work.

This note turns Phase 2 of `docs/lsp/03-delivery/roadmap.md` into a concrete crate shape.

Its job is to define the first reusable semantic layer that sits:

- above the existing parser crate
- below the future LSP crate
- outside editor-specific packaging such as the Zed extension

The aim is not to implement every semantic rule immediately.

The aim is to make the first implementation pass start from stable crate boundaries, stable types, and a testable public surface.

## Why this crate needs to exist

The current parser crate already gives us:

- the Tree-sitter language
- static node-type metadata
- portable query exports when query files exist

But it does **not** yet give us a reusable semantic surface for:

- syntax diagnostics as stable data
- extracted `!include` facts
- extracted `!identifiers` facts
- symbol and reference facts
- future workspace-instance indexing

Putting all of that directly in the future LSP crate would make protocol wiring and semantic modeling too tightly coupled.

So the analysis crate should become the first place where “Structurizr editor semantics” live in a transport-agnostic way.

## Current repo constraints

This repository already has an important packaging shape:

- the root `Cargo.toml` is the parser crate package
- the parser crate's library entry point is `crates/structurizr-grammar/bindings/rust/lib.rs`
- there is **not** a separate `crates/structurizr-grammar/bindings/rust/Cargo.toml`
- the repository root must remain a normal Tree-sitter grammar repo for Zed and other grammar consumers

That means the analysis crate should be added **alongside** the existing parser crate, not by moving the parser into a new nested Cargo package.

## Recommended Cargo/workspace shape

The recommended near-term layout is:

```text
Cargo.toml                      existing parser crate package at repo root
crates/structurizr-grammar/bindings/rust/lib.rs            existing parser crate entry point
crates/structurizr-analysis/    new analysis crate
```

The root `Cargo.toml` should eventually become a combined package + workspace:

```toml
[package]
name = "tree-sitter-structurizr"
# existing parser package fields stay here

[workspace]
resolver = "2"
members = ["crates/structurizr-analysis"]

[workspace.dependencies]
tree-sitter = "0.26.7"
insta = "1.43"
rstest = "0.25"
```

And the new crate should look roughly like:

```toml
[package]
name = "structurizr-analysis"
edition = "2021"

[dependencies]
tree-sitter = { workspace = true }
tree-sitter-structurizr = { path = "../structurizr-grammar" }

[dev-dependencies]
insta = { workspace = true }
rstest = { workspace = true }
indoc = "2.0"
```

Important implications:

- the parser crate stays exactly where Zed expects it
- `cargo build`, `cargo nextest run`, and the existing `Justfile` commands can keep running from repo root
- the analysis crate can depend directly on the checked-in parser crate without introducing a repo split

If package naming needs refinement before publication, that can be revisited later.

The important thing now is the crate boundary and workspace shape, not the final crates.io naming decision.

## Responsibility split

The analysis crate should own:

- parse orchestration over the existing grammar crate
- immutable per-document snapshots
- syntax-diagnostic extraction
- extracted directive facts such as `!include` and `!identifiers`
- extracted symbol and reference facts for the bounded MVP
- future workspace-instance graph and include-resolution facts

The parser crate should continue to own:

- `LANGUAGE`
- `NODE_TYPES`
- portable queries such as highlights and future tags
- grammar generation/build details

The future LSP crate should own:

- protocol handlers
- document sync state
- workspace folder registration
- file watching/orchestration
- conversion between analysis facts and `lsp-types`

This means the analysis crate should **not** depend on:

- `tower-lsp-server`
- `lsp-types`
- `tokio`
- editor-specific query surfaces from `zed-structurizr`

## Dependency policy

The first version of the analysis crate should stay deliberately lean.

### Add immediately

- `tree-sitter`
- `tree-sitter-structurizr`

### Add when workspace indexing begins

- `ignore`

This belongs in the analysis crate once Phase 3 starts, because workspace scanning and include resolution are analysis concerns rather than LSP-transport concerns.

### Defer unless implementation pressure proves it necessary

- `line-index`
- `ropey`
- `serde`
- `parking_lot`
- `dashmap`
- `thiserror`

None of those should be treated as part of the crate skeleton by default.

Tree-sitter byte ranges and points are already enough for the first diagnostics and fact extraction slice.

## Public API design rules

The first public surface should stay simple and heavily owned.

### Rule 1: public facts should be owned, not borrowed from Tree-sitter nodes

Do **not** expose public types that borrow `tree_sitter::Node<'tree>`.

That would make snapshots awkward to store, awkward to test, and awkward to move between layers.

Instead:

- keep the `Tree` inside the snapshot
- expose owned facts with ranges, kinds, names, and raw text fragments
- provide `tree()` accessors only for advanced internal/debug use

### Rule 2: make immutable document snapshots the main exchange object

The core thing the crate returns should be a `DocumentSnapshot`.

That snapshot should represent one parse/analyze pass over one document and should carry:

- the document identity
- the source text
- the Tree-sitter tree
- syntax diagnostics
- extracted directives/facts
- extracted symbols and references for the bounded MVP

### Rule 3: use transport-agnostic locations

Ranges should be represented in crate-owned types, not LSP ranges.

A small local span model is enough:

- byte range
- start point
- end point

That can later be converted into editor/protocol-specific location types in the LSP crate.

### Rule 4: keep document identity opaque, with filesystem location separate

The crate should not assume every document is only identified by a file path.

For example:

- LSP open documents are naturally URI-keyed
- workspace discovery is naturally path-keyed

The clean compromise is:

- an opaque `DocumentId`
- optional file-backed location metadata carried separately

That keeps the crate transport-agnostic without making local include resolution impossible.

### Rule 5: prefer synchronous analysis APIs first

Parsing and fact extraction are CPU-bound and deterministic.

The first crate surface should therefore stay synchronous.

Async orchestration belongs in the LSP or host layer if it is needed later.

### Rule 6: do not freeze incremental parsing into the first public contract

The first public API should accept full source text and return a new snapshot.

If later work wants to reuse previous trees for incremental parsing, that can be added behind the same snapshot-oriented API.

## Recommended initial module layout

The roadmap already sketched a coarse Phase 2 layout.

A more concrete starting point is:

```text
crates/structurizr-analysis/
  Cargo.toml
  src/lib.rs
  src/parse.rs
  src/snapshot.rs
  src/span.rs
  src/diagnostics.rs
  src/includes.rs
  src/symbols.rs
  src/workspace.rs
  src/extract/
    mod.rs
    diagnostics.rs
    includes.rs
    symbols.rs
  tests/
    document_snapshots.rs
```

### Why this shape

- `parse.rs` owns parser setup and tree creation
- `snapshot.rs` owns immutable per-document results
- `span.rs` prevents location logic from leaking raw tuple/`Range<usize>` shapes everywhere
- `diagnostics.rs` owns syntax-diagnostic types first
- `includes.rs` owns raw `!include` facts now and resolution types later
- `symbols.rs` owns bounded-MVP symbols, references, and identifier-mode facts
- `workspace.rs` is the natural home for later workspace-instance types
- `extract/` keeps tree-walking logic private and feature-sliced

The concrete design for that later workspace layer is captured in `docs/lsp/02-design/workspace-index.md`.

## A note on `queries.rs`

The roadmap's first sketch mentioned `queries.rs`.

That should stay **private or absent** until a real portable query surface exists for the analysis task at hand.

In practice:

- the initial symbol and directive extractors should be handwritten tree walks based on the audit docs
- when `crates/structurizr-grammar/queries/tags.scm` lands, a private query helper can be introduced without forcing the whole analysis architecture to depend on queries from day one

This keeps the skeleton honest about the current repo state.

## Core public types to define first

The first pass should settle a stable type set even if not every type is fully used immediately.

| Type | Purpose | First used in |
| --- | --- | --- |
| `DocumentId` | Opaque identifier for one analyzed document | Phase 2 |
| `DocumentLocation` | Optional file-backed location metadata for local include resolution | Phase 2 |
| `DocumentInput` | Full-text input to analysis | Phase 2 |
| `TextSpan` | Owned location data using bytes + points | Phase 2 |
| `DocumentSnapshot` | Immutable parse + fact result for one document | Phase 2 |
| `SyntaxDiagnostic` | Parse-error fact with stable ranges and messages | Phase 2 |
| `IncludeDirective` | Raw `!include` fact with raw value text/range and container context | Phase 2 |
| `IdentifierModeFact` | Raw `!identifiers` fact with declared mode and scope/container | Phase 2 |
| `Symbol` | Definition-like fact for bounded-MVP symbol extraction | Phase 2 |
| `Reference` | Reference-like fact for bounded-MVP syntax sites, resolved later | Phase 2 |
| `ResolvedInclude` | Include edge after path/url classification and local resolution | Phase 3 |
| `WorkspaceInstance` | Rooted multi-file semantic expansion | Phase 3 |

## Important modeling decisions

### `Reference` should mean “observed reference site”, not “successfully resolved binding”

This is an important layering decision.

The single-document analysis phase can observe reference sites before any workspace graph exists.

So `Reference` should capture things like:

- raw text
- syntax kind
- source range
- local container/context

But it should **not** require a resolved symbol target yet.

Resolution can be layered later with a workspace-instance pass.

### `IncludeDirective` should be raw syntax first

The directive audit already established that `!include` values are permissive at syntax time.

So the first `IncludeDirective` type should record:

- the raw directive value text
- its source range
- its container/scope
- the raw value kind (`string`, `identifier`, `bare_value`, etc.)

And **not** try to pretend that parsing already tells us whether the target is:

- a file
- a directory
- a URL
- invalid

That classification belongs in later resolution types such as `ResolvedInclude`.

### `IdentifierModeFact` should exist before rename/completion logic

The future LSP's rename and completion behavior will depend on `!identifiers`.

So the analysis crate should model that directive explicitly from the start instead of smuggling it indirectly through symbol tables later.

### Symbol kinds should be domain-shaped, not just raw node-kind strings

The analysis crate should expose concepts like:

- `Person`
- `SoftwareSystem`
- `Container`
- `Component`
- `Relationship`

That is more useful to later editor features than making every consumer decode raw node kinds itself.

Keeping an internal or debug-only `node_kind` string is fine, but the public API should prefer DSL concepts.

## Recommended public entrypoints

The first public API should be centered on one analyzer type and one convenience function.

Something roughly like:

```rust
pub struct DocumentAnalyzer {
    // private parser state
}

impl DocumentAnalyzer {
    pub fn new() -> Self;
    pub fn analyze(&mut self, input: DocumentInput) -> DocumentSnapshot;
}

pub fn analyze_document(input: DocumentInput) -> DocumentSnapshot;
```

The exact names can change.

What matters is:

- callers do not manage Tree-sitter parser setup themselves
- the result is an immutable snapshot
- tests can use a small, synchronous API directly

When Phase 3 starts, a separate `WorkspaceBuilder` or `WorkspaceAnalyzer` can layer on top of this single-document API rather than replacing it.

## First extracted fact surface

The current design note for this extraction slice is `docs/lsp/02-design/first-pass-symbol-extraction.md`.

The first implementation slice should only extract facts we have already audited and fixture-covered:

- syntax diagnostics from parse errors
- raw `!include` directives
- raw `!identifiers` directives
- top-level assigned identifiers
- direct declaration symbols for core element kinds
- direct identifier references in obvious syntax sites
- named relationship definitions

It should explicitly defer:

- `this`
- hierarchical selectors such as `a.b.c`
- dynamic-view relationship references as semantic bindings
- rename/edit planning

That keeps the crate aligned with the bounded MVP already documented elsewhere.

## Testing strategy

The analysis crate should reuse the repository's existing testing style rather than inventing a second one.

### Reuse existing DSL fixtures

Continue treating repo-root fixtures as the canonical DSL inputs:

- `crates/structurizr-grammar/tests/fixtures/`
- `crates/structurizr-grammar/tests/fixtures/lsp/`

Do **not** duplicate the same DSL files under the crate directory.

### Add crate-local analysis tests

Use crate-local Rust integration tests for extracted facts:

```text
crates/structurizr-analysis/tests/
  document_snapshots.rs
```

These tests should:

- load DSL text from the shared repo-root fixtures
- call the analysis crate API
- snapshot the extracted facts, not the raw parse tree sexp

### Keep analysis snapshots distinct from parser snapshots

The parser crate already snapshots raw trees.

The analysis crate should snapshot higher-level outputs such as:

- syntax diagnostics
- include facts
- identifier-mode facts
- symbol facts
- reference facts

That will keep test failures readable and avoid churn where one parser change rewrites both raw-tree and higher-level snapshots without clear separation.

### Prepare for later workspace tests

When workspace indexing begins, add scenario-focused workspace tests rather than stuffing multi-file behavior into the single-document fixture test.

That should align with the future shape already described in `docs/lsp/02-design/workspace-discovery-includes.md`.

## What this crate should not freeze too early

The skeleton should deliberately avoid overcommitting on:

- exact crates.io package names
- query-backed extraction as a requirement
- async APIs
- incremental-parse API shape
- final workspace-index invalidation strategy
- rename/conflict-resolution behavior
- full runtime-style validation

Those are real future concerns, but they are not required to get the first transport-agnostic semantic layer right.

## Recommended implementation sequence

1. Add the Cargo workspace metadata without moving the parser crate.
2. Create `crates/structurizr-analysis/` with `lib.rs` plus the core type modules.
3. Implement single-document parsing and syntax-diagnostic extraction.
4. Implement raw `!include` and `!identifiers` fact extraction.
5. Implement bounded-MVP symbol/reference extraction from the audited node shapes.
6. Add snapshot tests for extracted facts.
7. Only then begin workspace-instance and include-resolution logic.

This sequence keeps the crate useful early while preserving a clean path into the workspace-indexing phase.

## What this unblocks

Once this crate skeleton exists, future work no longer needs to answer “where should this logic live?” for every semantic feature.

It gives us:

- a home for syntax diagnostics that is not the parser crate and not the LSP crate
- a stable place to model symbols, references, and directives
- a clean seam for the later workspace-instance and include-graph work
- a transport-agnostic API the future Zed-facing LSP can consume directly

## Sources

- `Cargo.toml`
- `crates/structurizr-grammar/bindings/rust/lib.rs`
- `crates/structurizr-grammar/bindings/rust/build.rs`
- `Justfile`
- `README.md`
- `docs/lsp/03-delivery/roadmap.md`
- `docs/lsp/01-foundations/overview.md`
- `docs/lsp/01-foundations/capability-matrix.md`
- `docs/lsp/90-history/syntax-audit-assignment-declarations.md`
- `docs/lsp/90-history/syntax-audit-reference-relationship-nodes.md`
- `docs/lsp/90-history/syntax-audit-directive-nodes.md`
- `docs/lsp/02-design/workspace-discovery-includes.md`
