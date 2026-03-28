# Contributing

Thanks for helping improve `tree-sitter-structurizr`.

This repository is a Tree-sitter grammar for the Structurizr DSL. Contributions should optimize for faithful syntax structure, understandable parse trees, and stable coverage for the grammar, queries, and bindings surface rather than trying to turn the project into a Structurizr runtime.

## Prerequisites

For normal grammar and test work, install:

- a regular Rust toolchain
- [`just`](https://github.com/casey/just)
- the Tree-sitter CLI
- `cargo-nextest`

For performance benchmarking only, also install:

- [`hyperfine`](https://github.com/sharkdp/hyperfine) for black-box CLI timing
- [`uv`](https://docs.astral.sh/uv/) to run the replay helper with a pinned Python version

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
just bench-rust
just bench-black-box
just bench-perf
just bench-perf-stable
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

## Performance benchmarking

The benchmark surface deliberately tracks a small fixed matrix so performance
history stays comparable over time:

- small document: `tests/fixtures/lsp/identifiers/direct-references-ok.dsl`
- medium document: `tests/lsp/workspaces/big-bank-plc/model/people-and-software-systems.dsl`
- large document: `tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl`
- small workspace: `tests/lsp/workspaces/minimal-scan`
- medium workspace: `tests/lsp/workspaces/directory-include`
- large workspace: `tests/lsp/workspaces/big-bank-plc`
- small LSP session: `tests/fixtures/lsp/relationships/named-relationships-ok.dsl`
- large LSP session: `tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl`

Useful commands from the repository root:

```sh
just bench-rust
just bench-black-box
just bench-perf
just bench-perf-stable
```

Use `just bench-rust` when you want the in-process analysis and LSP benchmark
suite without any external tooling beyond Cargo. Use `just bench-black-box`
when you want Hyperfine measurements for `strz check`, `strz dump workspace`,
and a replayed `strz server` session against the checked-in fixtures. Use
`just bench-perf` or `just bench-perf-stable` when you want the combined flow
plus environment capture written to `tmp/benchmark-results/`.

The stable mode is still best-effort rather than perfectly reproducible. On
Linux, you can set `STRZ_BENCH_CPUSET=2` (or another CPU set) before running
`just bench-perf-stable` to request CPU pinning via `taskset`. On macOS, the
same command still captures environment metadata, but it cannot offer the same
level of scheduler control.

The LSP replay helper prefers `uv run --python 3.12 tools/lsp_replay.py` so
the interpreter version stays explicit, and falls back to `python3` only when
`uv` is not available. If you want to sanity-check the CodSpeed-compatible
bench harness locally, run:

```sh
cargo codspeed build -p structurizr-analysis -p structurizr-lsp
cargo codspeed run -p structurizr-analysis
cargo codspeed run -p structurizr-lsp
```

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

## Property-test workflow

The repository now keeps Proptest's default failure persistence enabled for the
property suites in `tests/property_parser.rs`,
`crates/structurizr-analysis/tests/property_analysis.rs`, and
`crates/structurizr-analysis/tests/property_workspace.rs`.

That means:

- failing cases are persisted under `proptest-regressions/`
- later runs automatically replay those persisted failures first
- you can pin a specific RNG seed with `PROPTEST_RNG_SEED`
- you can raise the case count with `PROPTEST_CASES`

Useful wrappers from the repository root:

```sh
just test-proptest --test property_parser
just test-proptest-stress 10000 --test property_workspace
just rerun-proptest 123456 --test property_analysis mutated_generated_workspaces_report_syntax_errors
just rerun-and-capture-proptest 123456 tmp/proptest-captures --test property_workspace generated_workspaces_load_idempotently
```

Capture notes:

- Set `STRUCTURIZR_PROPTEST_CAPTURE_DIR` (or use `just capture-proptest` / `just rerun-and-capture-proptest`) when you want the currently generated case materialized on disk.
- For single-document properties, the capture directory receives one `.dsl` file per property test name.
- For generated workspace properties, the capture directory receives one subdirectory per property test name containing the generated workspace tree.
- For the cleanest seeded replay, prefer `PROPTEST_CASES=1` when capturing a specific seed so the output directory reflects a single generated case.

Promotion notes:

- Start by capturing into `tmp/proptest-captures/`.
- If the minimized case is worth keeping, promote it into `tests/fixtures/`, `tests/lsp/workspaces/`, or `test/corpus/` depending on whether it is best represented as a realistic fixture, a workspace discovery regression, or a small syntax teaching example.
- Commit any useful `proptest-regressions/` updates alongside the curated fixture or corpus promotion when they still add value.

## Grammar fuzzing

Use `tree-sitter fuzz` as the current complementary fuzzing layer for the
grammar. It mutates the checked-in corpus inputs with random edits and checks
that incremental parse behavior stays consistent.

Useful wrappers from the repository root:

```sh
just fuzz-grammar
just fuzz-grammar 25 4
just fuzz-grammar-stress
```

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
