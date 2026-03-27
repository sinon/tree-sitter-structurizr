# Structurizr DSL LSP overview

## Why this is feasible

Yes: this repository is already a strong foundation for a future Structurizr DSL language server.

The current codebase already provides:

- a Tree-sitter grammar in `grammar.js`
- generated parser artifacts in `src/`
- a Rust language crate in `bindings/rust/`
- checked-in editor queries in `queries/`
- fixture and corpus coverage for real `.dsl` files

That means we do **not** need to invent a new parser, tokenizer, or editor-facing syntax model. The grammar crate can stay the parsing layer, while a future LSP adds semantic analysis, indexing, and protocol handling on top.

## Current repository state

The repository is intentionally parser-first, not runtime-first.

Important constraints from the current project shape:

- the goal is editor support, not executing the Structurizr DSL
- syntax highlighting, folds, and indentation already exist as checked-in Tree-sitter query artifacts
- Rust is the only shipped binding today
- unsupported executable directives such as `!script` and `!plugin` are intentionally out of scope

This is a good fit for an LSP aimed at Zed and similar editors, because an editor-oriented server can stop at:

- syntax-aware diagnostics
- workspace indexing
- symbol/navigation features
- reference and rename support
- completion and hover

without turning into a full Structurizr runtime.

## What Tree-sitter gives us

The existing grammar and Rust bindings should be reused directly for:

- incremental parsing of open documents
- syntax-error detection via `ERROR` and `MISSING` nodes
- stable node kinds for document structure
- byte/range mapping for LSP diagnostics and navigation
- query-driven editor features where that is cheaper than handwritten traversal

The existing query files are already useful for editor integration outside the LSP:

- `queries/highlights.scm`
- `queries/folds.scm`
- `queries/indents.scm`

The Rust bindings also already have an ergonomic surface for embedding the language:

- `LANGUAGE`
- `NODE_TYPES`
- conditional query exports

One notable gap: the Rust bindings can expose additional query constants if `injections.scm`, `locals.scm`, or `tags.scm` are added later, but those files are not present today. Future query-driven symbol/navigation work is possible without changing the overall crate shape, but those query files would need to be created explicitly.

## What Tree-sitter does not give us

Tree-sitter is the syntax layer, not the semantic layer.

A useful Structurizr DSL LSP will still need to build:

- a document store
- a workspace/include graph
- symbol tables for identifiers and named relationships
- reference resolution across files and scopes
- diagnostics beyond syntax, such as unresolved identifiers and duplicate definitions
- editor-friendly projections like hovers, completions, and rename edit sets

In other words: Tree-sitter gets us to a faithful parse tree very quickly, but navigation and validation still need a purpose-built analysis layer.

## Recommended architecture

### Recommendation

Start by keeping the first LSP implementation in this repository as additional Rust crates in a workspace, rather than creating a separate repository immediately.

Why:

- grammar evolution and LSP evolution will likely move together for a while
- the existing fixtures and upstream audit are already the best source of syntax truth
- local development is simpler if the server depends directly on the checked-in parser crate
- a Zed extension can still consume the grammar and launch the LSP as separate deliverables later

For the repo-topology tradeoffs behind that recommendation, see `docs/lsp/01-foundations/repository-topology.md`.

If the server later grows a substantially different release cadence, packaging story, or contributor base, it can be split out after the interfaces stabilize.

### Suggested crate layout

Suggested future shape:

- `bindings/rust/` or top-level crate: parser language crate as it exists today
- `crates/structurizr-analysis/`: typed analysis facade over Tree-sitter nodes
- `crates/structurizr-lsp/`: LSP transport, request handling, and workspace coordination
- optional later `crates/structurizr-test-support/`: shared helpers for LSP and analysis fixtures

### Suggested analysis pipeline

1. Store document text in `ropey`, plus a line index for LSP position conversions.
2. Reuse the previous Tree-sitter tree on edits for incremental parsing.
3. Convert syntax errors into basic diagnostics immediately.
4. Walk the tree into a thin semantic index:
   - definitions
   - references
   - include directives
   - views and view targets
   - relationship declarations
5. Merge per-file indexes into a workspace graph.
6. Answer LSP requests from the workspace graph.

This keeps the parser and semantic layers separate:

- Tree-sitter stays the source of truth for syntax
- analysis code stays focused on Structurizr concepts
- the LSP stays mostly protocol glue and cache orchestration

## Dependencies and tooling to reuse

## LSP transport

Recommended first choice:

- `tower-lsp`

Why:

- high-level Rust LSP scaffolding
- async-friendly
- lower boilerplate for a greenfield server
- easier to get an MVP server standing up quickly

Alternative:

- `lsp-server` + `lsp-types`

Why you might choose it later:

- more manual control
- closer to the style used by lower-level Rust tools such as `rust-analyzer`

Recommendation: begin with `tower-lsp`, and only drop lower-level if future performance or control needs justify it.

## Core analysis/runtime

Recommended building blocks:

- `tree-sitter` for incremental parsing and queries
- existing `tree-sitter-structurizr` crate from this repository
- `tokio` for async runtime if using `tower-lsp`
- `lsp-types` directly in the LSP crate for protocol data types
- `ropey` for efficient document text updates
- `line-index` for byte/line/UTF-16 position conversions
- `ignore` for workspace scanning of `.dsl` files
- `serde` / `serde_json` where needed for config or test fixtures
- `tree-sitter::Query` and `QueryCursor` directly for query-backed extraction
- existing `rstest` + `insta` patterns, extended with async/protocol fixtures

Useful but optional:

- `parking_lot` if lighter locking becomes useful after the initial design settles
- `dashmap` only if later profiling shows real contention pressure
- `petgraph` if relationship or include graphs become complex enough to benefit from explicit graph algorithms

## Tooling to avoid building ourselves

We should **not** reinvent:

- parsing
- syntax highlighting
- folding/indent queries
- the LSP transport loop
- filesystem walking
- common rope/text-buffer behavior

The work that remains genuinely project-specific is:

- modeling Structurizr scopes and references
- resolving `!include` graphs
- deciding how `!identifiers` should shape completion and rename behavior
- understanding identifier assignment and selector usage
- mapping DSL concepts to editor features

## Feature layering

The cleanest approach is to split features into three layers.

### Layer 1: syntax-backed features

These can ship early and cheaply:

- syntax diagnostics
- selection ranges
- document symbols
- keyword/directive completion
- basic hover from local syntax context

### Layer 2: single-workspace semantic features

These require a semantic index:

- go to definition
- find references
- rename
- identifier completion
- duplicate/unresolved diagnostics
- workspace symbols

### Layer 3: richer editor features

These should wait until the semantic model is stable:

- semantic tokens
- code actions
- include resolution and diagnostics
- cross-file refactors
- richer hover for view/model relationships

## Zed integration

Zed already expects language support to be split across:

- language metadata/config
- grammar
- queries
- language servers

That maps well to this project.

Recommended integration shape:

1. A Zed extension registers the Structurizr grammar from this repository.
2. The extension ships or references the query files for syntax highlighting, folding, indentation, and other editor-native features.
3. The extension launches the future Structurizr LSP binary.
4. Zed combines Tree-sitter syntax support with LSP features.

Important consequence: the first version of the LSP does **not** need to replace query-driven editor features. Zed can already use Tree-sitter for many editor behaviors, so the LSP should focus on semantic value first.

Semantic tokens should therefore be a later addition, not an MVP requirement.

## Repository topology and Zed build constraints

There is already a separate Zed extension repository at `/Users/rob/dev/zed-structurizr`.

That matters because today the extension:

- pins this grammar repository directly in `extension.toml`
- keeps its own Zed-side language config and query files under `languages/structurizr/`
- already carries Zed-specific query surfaces such as `brackets.scm`, `outline.scm`, and `textobjects.scm`

So the extension is **not** just consuming the Rust bindings. It is consuming this repository as a grammar repository in the way Zed expects.

That creates an important packaging constraint:

- the grammar must remain independently buildable from the repository root
- the extension cannot assume it can build the grammar from an arbitrary crate subdirectory
- the LSP should not force the grammar to stop looking like a normal Tree-sitter grammar repository

Recommended near-term structure:

- keep grammar + future analysis crate + future LSP crate in this repository
- keep the Zed extension in its own repository as a downstream packaging layer
- let the extension pin grammar revisions intentionally, rather than forcing every grammar/LSP commit through extension release flow

For local development, prefer:

- a `file://` grammar repository override in the dev extension when iterating on grammar changes
- a locally built LSP binary path in the extension while iterating on the server

That gives fast grammar/LSP co-development without requiring every iteration to become a cross-repo release.

One more design choice to make deliberately: the grammar repo and the Zed extension repo do not currently own exactly the same query surface. Future work should decide which queries are canonical in the grammar repo versus extension-owned for Zed specifically.

## Biggest technical gaps to solve

The main missing pieces are not parser mechanics but semantic modeling decisions:

- how `!include` should be resolved and indexed, starting with file resolution before richer semantics
- how identifier scopes behave in nested blocks
- how relationship identifiers and dynamic-view references are represented
- how selectors and hierarchical names resolve
- how `!identifiers` should affect completion and rename behavior
- how much validation should be editor-helpful versus upstream-complete

The more explicitly these rules are documented in the analysis layer, the less likely the LSP is to drift into becoming an unofficial runtime.

## Recommended MVP

A realistic first server should aim for:

- initialize/shutdown/text sync
- syntax diagnostics
- document symbols
- go to definition for straightforward identifiers
- find references for straightforward identifiers
- keyword/directive completion
- workspace indexing for `.dsl` files and first-pass `!include` file resolution

For MVP, “straightforward identifiers” should mean:

- top-level assigned identifiers such as `a = softwareSystem "A"`
- direct references to model element identifiers in obvious cases

It should explicitly exclude until later phases:

- `this`
- hierarchical selectors such as `a.b.c`
- dynamic-view relationship references
- rename logic that depends on unresolved scope questions

That would already make the language meaningfully usable in Zed and other editors.

## Diagnostics strategy

The server should treat diagnostics in layers:

1. Always report syntax diagnostics from Tree-sitter parse errors.
2. Only report semantic diagnostics when the surrounding syntax is parseable enough to trust.
3. Prefer file-resolution diagnostics first for `!include` handling, before deeper include semantics.

This keeps the early server helpful without pretending it can fully validate broken or runtime-dependent documents.

## Non-goals for the first implementation

- executing `!script` or `!plugin`
- reproducing every upstream semantic rule before shipping anything useful
- replacing Tree-sitter-based highlighting/folding behavior inside editors
- building a full Structurizr runtime
- validating every runtime-level Structurizr rule that the upstream Java implementation enforces
- turning hover or diagnostics into a proxy for the Structurizr runtime or hosted platform

## References

- Repository source of truth: `README.md`, `CONTRIBUTING.md`, `grammar.js`, `bindings/rust/lib.rs`, `bindings/rust/build.rs`, `queries/`
- Existing Zed extension context: `/Users/rob/dev/zed-structurizr`
- Zed language support docs: <https://zed.dev/docs/extensions/languages>
- `tower-lsp`: <https://docs.rs/tower-lsp/latest/tower_lsp/>
- `lsp-server`: <https://docs.rs/lsp-server/latest/lsp_server/>
- `tree-sitter` Rust bindings: <https://docs.rs/tree-sitter/latest/tree_sitter/>
