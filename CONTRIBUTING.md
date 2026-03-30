# Contributing

This repository now has three areas of focus:

- grammar and query coverage work
- transport-agnostic analysis and workspace work
- LSP, CLI, and downstream editor-integration work

If you are starting on the semantic or editor-facing side, read `docs/lsp/00-current-state.md` first. It is the quickest summary of the current architecture, shipped bounded surface, and remaining path to feature completeness.

## Choosing a starting point

Pick the layer before you pick the code:

- grammar or query work: start with `README.md`, then use this file's recommended grammar workflow and test surfaces
- analysis or LSP work: start with `docs/lsp/00-current-state.md`, then `docs/lsp/01-foundations/overview.md`, then the relevant design note under `docs/lsp/02-design/`
- downstream editor wiring or release work: start with `docs/lsp/03-delivery/roadmap.md` and the packaging/wiring notes under `docs/lsp/03-delivery/`

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

The [`Justfile`](Justfile) has the canonical command surface:

Recommended baseline loop for grammar development:

```sh
just generate
just test-grammar
INSTA_UPDATE=always just test-rust
```

When you change docs, run `just check-links` to verify relative markdown links
and fragment anchors across `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, and
`docs/**`.

Use `just audit-upstream` when you are hardening coverage against upstream Structurizr examples. It is useful for maintainers and focused grammar work, but it is not required for every consumer-facing change.

## Analysis, LSP, and CLI workflow

The workspace now includes `strz`, the local entrypoint for both
`structurizr-analysis` and the in-repo stdio language server.

Use it when you want to inspect extracted facts, reproduce diagnostics without
an editor, or launch the same `strz server` entrypoint that downstream
integrations should execute.

Useful commands from the repository root:

```sh
just build-strz
just test-cli
cargo test -p structurizr-lsp --test navigation
just run-strz check
just run-strz dump workspace tests/lsp/workspaces/directory-include
just run-strz dump document tests/fixtures/lsp/identifiers/direct-references-ok.dsl
just run-strz server
```

If you are changing semantic behavior, read `docs/lsp/00-current-state.md`
first, then `docs/lsp/01-foundations/overview.md`, and then the specific
design note for the slice you are touching.

Use `check` when you want aggregated syntax and include diagnostics for a file
or workspace. Use `dump document` and `dump workspace` when you want to inspect
the extracted analysis facts that sit underneath the LSP. Use `server` when you
want to run the same stdio language server entrypoint that editor integrations
should launch.

### Observability for local debugging

The CLI and LSP test harness now support opt-in tracing so deadlocks or
unexpected `null` results are easier to inspect without temporary `println!`
debugging.

Useful patterns from the repository root:

```sh
RUST_LOG=info just run-strz server
RUST_LOG=debug STRZ_LOG_FORMAT=json STRZ_LOG_FILE=tmp/strz-server.log just run-strz server
STRZ_TEST_LOG=1 RUST_LOG=info cargo test -p structurizr-lsp --test navigation goto_definition_returns_no_result_for_multi_instance_open_fragments
```

Use `RUST_LOG` to control verbosity, `STRZ_LOG_FORMAT=compact|json` to pick
human-readable versus machine-friendly output, and `STRZ_LOG_FILE=...` to write
logs to a file under `tmp/` instead of stderr. The `STRZ_TEST_LOG=1` helper
gives the LSP integration tests a deterministic `tmp/structurizr-lsp-tests.log`
artifact when you need to debug one hanging case.

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

For a more detailed workflow with targeted `cargo bench` filters, CodSpeed
parity commands, and profiler examples, see `docs/performance-testing.md`.

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
- `multi-line-with-error.dsl` is also ignored permanently as it is an intentionaly invalid multiline sample

The audit downloads sample `.dsl` files from the upstream [structurizr/structurizr](https://github.com/structurizr/structurizr) repository and reports:

- total checked / clean / failing
- breakdown by broad feature area
- extracted text for `ERROR` and `MISSING` nodes

## Generated artifacts

- `src/parser.c`
- `src/grammar.json`
- `src/node-types.json`
- `bindings/rust/**` - generated by `tree-sitter init` these are not re-generated by changes to `grammar.js`

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
