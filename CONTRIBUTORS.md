# Contributors

This repository's published surface is the Tree-sitter grammar and Rust bindings. Contributor-only workflow details live here so `README.md` can stay focused on the project itself.

## Toolchains

- Normal library and test work uses the regular Rust toolchain plus the existing Tree-sitter tooling.
- The upstream audit command is **development-only** and uses Cargo's unstable single-file package support via `-Zscript`.
- That means contributors need a nightly Cargo toolchain to run the audit workflow.

The unstable Cargo feature is not part of the published bindings surface and is not required for downstream consumers of `tree-sitter-structurizr`.

## Development commands

The `Justfile` is the canonical command surface:

```sh
just generate
just test-grammar
just test-rust
just test-rust-fast
just audit-upstream
```

## Upstream audit workflow

The upstream audit lives in `tools/upstream_audit.rs` as a single-file Rust package with an embedded manifest.

It is run through nightly Cargo's `-Zscript` support:

```sh
cargo +nightly -Zscript tools/upstream_audit.rs
```

The `just` wrappers are:

```sh
just audit-upstream
just audit-upstream-all
```

Useful variants:

```sh
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

## Repository layout for contributors

- `grammar.js` — source of truth for the grammar
- `src/parser.c`, `src/grammar.json`, `src/node-types.json` — generated artifacts
- `bindings/rust/lib.rs` — Rust bindings entry point
- `tests/fixtures.rs` — fixture-driven Rust tests with snapshots
- `tests/fixtures/` — main Rust fixture tree
- `test/corpus/` — Tree-sitter CLI corpus tests
- `tools/upstream_audit.rs` — contributor-only upstream audit script
- `queries/` — placeholder area for future highlighting/folding/indentation queries

## Recommended grammar workflow

When changing the grammar, use this loop:

1. Pick a narrow syntax slice from the upstream audit.
2. Read the failing upstream examples for that slice.
3. Add or adjust local coverage first.
4. Update `grammar.js`.
5. Regenerate parser artifacts with `just generate`.
6. Run local validation:
   - `just test-grammar`
   - `INSTA_UPDATE=always just test-rust`
7. Review snapshot changes carefully.
8. Re-run the upstream audit for the narrow slice, then more broadly.

## Attribution

The upstream audit uses sample DSL files from the upstream Structurizr repository. Those samples are not vendored into this repository. They remain upstream project materials and are used here under the upstream project's Apache-2.0 license. See the upstream [LICENSE](https://github.com/structurizr/structurizr/blob/main/LICENSE) for the governing terms.
