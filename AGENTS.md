# AGENTS.md

## Purpose of this repository

This repository contains a Tree-sitter grammar for the Structurizr DSL, with Rust bindings checked in for downstream editor tooling.

The main goal is editor support rather than DSL execution:

- syntax highlighting
- code folding
- indentation/query support
- robust parsing of real `.dsl` files for a future Zed extension and other Rust-based consumers

This project is not trying to become a Structurizr runtime. It should preserve and expose syntax structure faithfully enough for editor features, tests, and iterative grammar hardening.

## Core files and layout

- `grammar.js` — source of truth for the grammar
- `src/parser.c`, `src/grammar.json`, `src/node-types.json` — generated artifacts
- `tests/snippets.rs` — focused inline Rust parser tests
- `tests/fixtures.rs` — fixture-driven Rust tests with snapshots
- `tests/common/mod.rs` — shared parser/test helpers and parse-issue extraction
- `tests/upstream_audit.rs` — ignored integration test that downloads upstream Structurizr DSL fixtures and audits parser coverage
- `test/corpus/` — Tree-sitter CLI corpus tests
- `tests/fixtures/pass/` — fixtures that should parse cleanly
- `tests/fixtures/future/` — fixtures intentionally kept as pending coverage
- `queries/` — placeholder area for future highlighting/folding/indentation queries
- `Justfile` — canonical command surface

## Test harnesses and why they exist

### 1. Tree-sitter corpus tests

Command:

```sh
just test-grammar
```

This runs `tree-sitter test` against `test/corpus/`.

Use this harness for:

- compact grammar regression tests
- validating named-node shape in a Tree-sitter-native format
- checking that representative syntax parses as expected after grammar edits

This is the fastest harness for parser structure changes, but it is intentionally smaller and narrower than the Rust suite.

### 2. Rust snippets and snapshot fixtures

Commands:

```sh
just test-rust
just test-rust-fast
```

This is the main local correctness harness.

It has two layers:

- `tests/snippets.rs` for focused inline examples
- `tests/fixtures.rs` for file-based fixtures under `tests/fixtures/`

The fixture tests snapshot parse trees with `insta`. This gives stable visibility into parse-tree shape changes over time, which is especially useful when a grammar change fixes one syntax family but accidentally reshapes another.

Use this harness for:

- asserting that supported syntax parses without `ERROR` nodes
- keeping parse-tree structure stable enough for future editor query work
- reviewing intended snapshot changes when node shapes evolve

When snapshots need to be updated intentionally, run:

```sh
INSTA_UPDATE=always just test-rust
```

Do not blindly accept snapshot churn. Check that the tree shape still makes sense and that changes are directly caused by the grammar work you intended.

### 3. Upstream audit harness

Command:

```sh
just audit-upstream
```

This runs the ignored Rust integration test in `tests/upstream_audit.rs`.

It downloads upstream Structurizr DSL fixtures from the Structurizr repository, parses them with the local grammar, and reports:

- total checked / clean / failing
- breakdown by broad feature area
- extracted text for `ERROR` and `MISSING` nodes

This harness is the main backlog generator for ongoing grammar coverage work.

Important behavior:

- fixtures whose path contains `unexpected-` are ignored permanently because they are upstream negative parser tests
- fixtures whose path contains `script` or `plugin` are excluded by default because those features are explicitly unsupported here

To include explicitly unsupported fixtures anyway:

```sh
just audit-upstream-all
```

To narrow the audit to a slice:

```sh
STRUCTURIZR_UPSTREAM_FILTER=deployment just audit-upstream
STRUCTURIZR_UPSTREAM_FILTER=archetypes just audit-upstream
```

## Expected workflow for an agent

When changing the grammar, use this loop:

1. Pick a narrow syntax slice from the upstream audit.
2. Read the failing upstream examples for that slice.
3. Add or adjust local coverage first:
   - snippet tests in `tests/snippets.rs`
   - fixture files under `tests/fixtures/pass/` or `tests/fixtures/future/`
   - corpus coverage under `test/corpus/` if the syntax belongs in the compact CLI suite
4. Update `grammar.js`.
5. Regenerate parser artifacts:

```sh
just generate
```

6. Run the local harnesses:

```sh
just test-grammar
INSTA_UPDATE=always just test-rust
```

7. Review snapshot changes carefully.
8. Run the upstream audit again for the narrow slice first, then more broadly:

```sh
STRUCTURIZR_UPSTREAM_FILTER=<slice> just audit-upstream
just audit-upstream
```

9. Update docs if support status changed:
   - `README.md`
   - this `AGENTS.md` if the workflow itself changed

## How to decide where a test belongs

Use `test/corpus/` when:

- the example is small
- the tree shape is important
- the syntax belongs in the stable Tree-sitter-native regression suite

Use `tests/snippets.rs` when:

- you want a focused parser assertion
- the syntax is easier to express inline
- you need a high-signal regression test for a single feature

Use `tests/fixtures/pass/` when:

- the example is more realistic or multi-block
- you want snapshot coverage for a representative DSL file

Use `tests/fixtures/future/` when:

- the syntax is intentionally still pending
- you want to keep a concrete example around without claiming support yet

If a future fixture starts parsing cleanly because the grammar expanded, move or rewrite it. Future fixtures are expected to continue producing parse errors.

## Current support model

Broadly supported today:

- core workspace/model/views/configuration structure
- comments, strings, identifiers, numbers
- core C4 model elements and relationships
- several view families
- styles
- core directives used by existing fixtures
- initial archetypes/custom elements
- a meaningful deployment slice

Explicitly unsupported:

- `!script`
- `!plugin`

Ignored upstream negatives:

- files containing `unexpected-`

Do not spend time implementing `!script` or `!plugin` unless the project scope changes explicitly.

## Progressive coverage strategy

The preferred way to improve the grammar is incremental and audit-driven:

1. Run `just audit-upstream`
2. Choose one broad bucket
3. Narrow to a smaller sub-slice by file or feature
4. Implement only enough syntax to make that slice parse well
5. Add local corpus/snippet/fixture coverage
6. Regenerate and update snapshots
7. Re-run the audit and measure the drop in failures

Avoid trying to “solve the DSL” in one pass. The grammar is intentionally being hardened in slices so node names, snapshots, and future editor queries remain understandable.

## Good handoff habits for the next agent

- Keep changes surgical and feature-scoped.
- Prefer adding representative fixtures from upstream examples over inventing synthetic mega-tests.
- If a grammar rule becomes too permissive, tighten it rather than silently reclassifying broken tests.
- When support moves from pending to implemented, update `README.md`.
- When the audit filtering policy changes, update both `tests/upstream_audit.rs` and `README.md`.
- Before ending work, leave the repo in a validated state:

```sh
just generate
just test-grammar
INSTA_UPDATE=always just test-rust
just audit-upstream
```

If `just audit-upstream` still fails, that is acceptable only if the remaining failures belong to features outside the slice you worked on. In that case, summarize the remaining buckets and call out what changed.
