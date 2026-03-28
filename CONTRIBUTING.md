# Contributing

Thanks for helping improve `tree-sitter-structurizr`.

This repository is a Tree-sitter grammar for the Structurizr DSL. Contributions should optimize for faithful syntax structure, understandable parse trees, and stable coverage for the grammar, queries, and bindings surface rather than trying to turn the project into a Structurizr runtime.

## Prerequisites

For normal grammar and test work, install:

- a regular Rust toolchain
- [`just`](https://github.com/casey/just)
- the Tree-sitter CLI
- `cargo-nextest`

For the upstream audit workflow only, you also need a nightly Cargo toolchain because the audit runs through `cargo +nightly -Zscript`.

## Canonical commands

The `Justfile` is the canonical command surface:

```sh
just generate
just test-grammar
just test-rust
just test-rust-fast
just build-strz
just test-cli
just audit-upstream
```

Recommended baseline loop:

```sh
just generate
just test-grammar
INSTA_UPDATE=always just test-rust
```

Use `just audit-upstream` when you are hardening coverage against upstream Structurizr examples. It is useful for maintainers and focused grammar work, but it is not required for every consumer-facing change.

## Analysis CLI workflow

The workspace now includes `strz`, a contributor-facing CLI that hosts
`structurizr-analysis` and the LSP server without requiring an editor loop.

Useful commands from the repository root:

```sh
just build-strz
just test-cli
just run-strz check
just run-strz dump workspace tests/lsp/workspaces/directory-include
just run-strz dump document tests/fixtures/lsp/identifiers/direct-references-ok.dsl
just run-strz server
```

Use `check` when you want aggregated syntax and include diagnostics for a file
or workspace. Use `dump document` and `dump workspace` when you want to inspect
the extracted analysis facts that sit underneath the LSP. Use `server` when you
want to run the same stdio language server entrypoint that editor integrations
should launch.

## Upstream audit workflow

The upstream audit lives in `tools/upstream_audit.rs` as a single-file Rust package with an embedded manifest.

It is run through nightly Cargo's `-Zscript` support:

```sh
cargo +nightly -Zscript tools/upstream_audit.rs
```

Useful wrappers and variants:

```sh
just audit-upstream
just audit-upstream-all
STRUCTURIZR_UPSTREAM_FILTER=deployment just audit-upstream
STRUCTURIZR_UPSTREAM_FILTER=archetypes just audit-upstream
STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 just audit-upstream
```

Behavior notes:

- fixtures whose path contains `unexpected-` are ignored permanently because they are intentional upstream negative tests
- fixtures whose path contains `script` or `plugin` are excluded by default because those features are explicitly unsupported here
- `multi-line-with-error.dsl` is also ignored permanently as an intentional invalid multiline sample

The audit downloads sample `.dsl` files from the upstream [structurizr/structurizr](https://github.com/structurizr/structurizr) repository and reports:

- total checked / clean / failing
- breakdown by broad feature area
- extracted text for `ERROR` and `MISSING` nodes

## What is edited vs generated

Hand-edited source:

- `grammar.js` — source of truth for the grammar
- `queries/*.scm` — checked-in highlighting, folding, and indentation queries
- `tests/fixtures/` — broader Rust fixture coverage
- `test/corpus/` — compact Tree-sitter-native regression coverage
- `README.md`, `CONTRIBUTING.md`

Generated artifacts:

- `src/parser.c`
- `src/grammar.json`
- `src/node-types.json`

When `grammar.js` changes, run `just generate` before testing or opening a PR.

## Test surfaces and how to use them

### `test/corpus/`

This is the cleanest place to see the DSL built up by concept. Prefer adding small, concept-focused grammar regressions here.

Current corpus layout:

| File | Main concepts |
| --- | --- |
| `smoke.txt` | minimal workspace envelope |
| `model.txt` | model elements, relationships, groups, deployment flow |
| `views.txt` | static views, filtered views, dynamic views, image/text sources |
| `styles.txt` | styles, finite style values, comments with style-heavy examples |
| `archetypes.txt` | archetypes, custom elements, selectors, grouped element updates |
| `workspace.txt` | workspace directives, extension forms, configuration, mixed workspace-level forms |

When adding a new concept, prefer making the corpus mapping clearer rather than adding another catch-all example if a narrow example will do.

### `tests/fixtures/`

This is the broader Rust snapshot suite. Keep using it for realistic, multi-block, or integration-style examples.

Important conventions:

- fixture filenames encode the expectation:
  - `-ok.dsl` means the file should parse without errors
  - `-err.dsl` means the file should continue to produce parse errors
- retaining broad snapshot-backed coverage is preferred over forcing every fixture into a perfect taxonomy

If a concept already has useful coverage inside a broader fixture, you do not need to split it out immediately unless discoverability or reviewability is suffering.

## Recommended grammar workflow

When changing grammar support:

1. Pick a narrow syntax slice from the upstream audit or a clearly scoped local gap.
2. Add or adjust local coverage first.
3. Update `grammar.js`.
4. Regenerate parser artifacts with `just generate`.
5. Run local validation:
   - `just test-grammar`
   - `INSTA_UPDATE=always just test-rust`
6. Review snapshot changes carefully.
7. Re-run the upstream audit for the affected slice when relevant.
8. Update docs if support status changed.

## Coverage map for contributors

The repository uses three complementary views of coverage:

- `test/corpus/` for gradual concept-oriented grammar coverage
- `tests/fixtures/` for richer snapshot-backed regression coverage
- `tools/upstream_audit.rs` as the backlog generator for broader Structurizr DSL parity work

If you are deciding where to add coverage, use this rule of thumb:

- choose `test/corpus/` when the concept should be easy to discover and teach
- choose `tests/fixtures/` when the example is more realistic, broader, or valuable as a regression snapshot
- use the upstream audit to choose the next syntax slice, not as a substitute for local tests

## Repository layout

- `grammar.js` — source of truth for the grammar
- `src/parser.c`, `src/grammar.json`, `src/node-types.json` — generated artifacts
- `bindings/rust/lib.rs` — Rust bindings entry point
- `tests/fixtures.rs` — fixture-driven Rust tests with snapshots
- `tests/fixtures/` — main Rust fixture tree
- `test/corpus/` — Tree-sitter CLI corpus tests
- `tools/upstream_audit.rs` — contributor-only upstream audit script
- `queries/` — checked-in highlighting/folding/indentation queries that are still expanding with grammar coverage

## Attribution

The upstream audit uses sample DSL files from the upstream Structurizr repository. Those samples are not vendored into this repository. They remain upstream project materials and are used here under the upstream project's Apache-2.0 license. See the upstream [LICENSE](https://github.com/structurizr/structurizr/blob/main/LICENSE) for the governing terms.
