# AGENTS.md

## Issue Tracking

This project uses **bd (beads)** for issue tracking.
Run `bd prime` for workflow context, or install hooks (`bd hooks install`) for auto-injection.

**Quick reference:**

- `bd ready` - Find unblocked work
- `bd create "Title" --type task --priority 2` - Create issue
- `bd close <id>` - Complete work
- `bd dolt push` - Push beads to remote

For full workflow details: `bd prime`

## Purpose of this repository

This repository contains an LSP and linter for the Structurizr DSL, built on top of tree-sitter grammar.

This project is not trying to become a Structurizr runtime. It should preserve and expose syntax structure faithfully enough for editor features, tests, and iterative grammar hardening.

## Current LSP work and boundaries

- [`docs/lsp/README.md`](docs/lsp/README.md) and [`docs/lsp/00-current-state.md`](docs/lsp/00-current-state.md) are the entry points for the current LSP architecture, status, and roadmap docs.
- This repo now already contains [`crates/strz-analysis/`](crates/strz-analysis/), [`crates/strz-lsp/`](crates/strz-lsp/), and [`crates/strz/`](crates/strz/) alongside the grammar.
- Keep `/Users/rob/dev/zed-structurizr` as a separate downstream editor integration repo.
- Treat the grammar as the syntax layer and the LSP as a separate semantic layer; do not distort grammar rules just to model runtime semantics.
- Prefer transport-agnostic analysis logic and keep protocol/editor glue thin.

## Core files and layout

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contributor workflow and canonical command surface
- [`crates/strz-grammar/grammar.js`](crates/strz-grammar/grammar.js) — source of truth for the grammar
- [`crates/strz-grammar/src/parser.c`](crates/strz-grammar/src/parser.c), [`crates/strz-grammar/src/grammar.json`](crates/strz-grammar/src/grammar.json), [`crates/strz-grammar/src/node-types.json`](crates/strz-grammar/src/node-types.json) — generated artifacts
- [`crates/strz-analysis/`](crates/strz-analysis/) — transport-agnostic document and workspace facts
- [`crates/strz-lsp/`](crates/strz-lsp/) — language server implementation
- [`crates/strz/`](crates/strz/) — `strz` CLI including `strz server`
- [`tools/upstream_audit.rs`](tools/upstream_audit.rs) — contributor-only single-file Cargo script for downloading upstream Structurizr DSL fixtures and auditing parser coverage
- [`crates/strz-grammar/tests/fixtures.rs`](crates/strz-grammar/tests/fixtures.rs) — fixture-driven Rust tests with snapshots
- [`crates/strz-grammar/test/corpus/`](crates/strz-grammar/test/corpus/) — Tree-sitter CLI corpus tests
- [`fixtures/`](fixtures/) — the main Rust fixture tree, organized by feature area
- [`crates/strz-lsp/tests/fixtures/`](crates/strz-lsp/tests/fixtures/) — LSP-specific single-document fixtures
- [`crates/strz-grammar/queries/`](crates/strz-grammar/queries/) — checked-in highlighting/folding/indentation queries
- [`docs/lsp/`](docs/lsp/) — current LSP architecture, status, and delivery docs
- [`Justfile`](Justfile) — canonical command surface

## Specification references

- `https://github.com/structurizr/structurizr.github.io/blob/main/dsl/71-language.md` - the language reference for the `.dsl`
- `https://github.com/structurizr/structurizr/tree/main/structurizr-dsl/src/main/java/com/structurizr/dsl` - the upstream dsl parser written in Java
- `https://github.com/structurizr/structurizr/tree/main/structurizr-dsl/src/test/resources/dsl` - the corpus of test `.dsl` files used to test the Java parser
- `https://github.com/structurizr/structurizr/blob/main/structurizr-core/src/main/java/com/structurizr/view/Shape.java` - the valid set of values for `shape`
- `https://github.com/structurizr/structurizr/blob/main/structurizr-core/src/main/java/com/structurizr/view/Color.java` - the valid set of colour names that upsteam parser supports.
- `https://github.com/structurizr/structurizr/tree/main/structurizr-core/src/main/java/com/structurizr/view` - holds a lot of static values (such as colour and shape others might be relevant for future gap filling)

### General development notes

- For ad-hoc debugging, create a temporary Rust example in examples/ and run it with cargo run --example <name>. Remove the example after use.
- Use tmp/ (project-local) for intermediate files and comparison artifacts, not /tmp. This keeps outputs discoverable and project-scoped. The tmp/ directory is gitignored.
- Use `gh` for fetching files from github instead of fetching web content.
- When you include a reference to a markdown doc in another markdown file include a fragment link so that lychee can catch drift
- Run `just check-links` after markdown edits.
- Use `hk fix` as the final mutating pass for repo hygiene and formatting. Keep it at the end of a workflow so agents do not need to re-read files after formatter churn.

## Test harnesses and why they exist

### 1. Tree-sitter corpus tests

Command:

```sh
just test-grammar
```

This runs `tree-sitter test` against [`crates/strz-grammar/test/corpus/`](crates/strz-grammar/test/corpus/).

Use this harness for:

- compact grammar regression tests
- validating named-node shape in a Tree-sitter-native format
- checking that representative syntax parses as expected after grammar edits

This is the fastest harness for parser structure changes, but it is intentionally smaller and narrower than the Rust suite.

### 2. Rust fixture snapshots

Commands:

```sh
just test-rust
just test-rust-fast
```

This is the main local correctness harness.

It is fixture-first:

- `fixtures.rs` loads file-based fixtures under [`fixtures/`](fixtures/)
- fixture filenames encode expectation:
  - `-ok.dsl` means the fixture should parse without errors
  - `-err.dsl` means the fixture should continue to produce parse errors

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

This runs the audit script in [`tools/upstream_audit.rs`](tools/upstream_audit.rs).

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
1. Read the failing upstream examples for that slice.
1. Add or adjust local coverage first:
   - fixture files under [`fixtures/`](fixtures/), organized by feature area
   - use `-ok.dsl` or `-err.dsl` suffixes to express expected outcome
   - corpus coverage under [`crates/strz-grammar/test/corpus/`](crates/strz-grammar/test/corpus/) if the syntax belongs in the compact CLI suite
1. Update [`crates/strz-grammar/grammar.js`](crates/strz-grammar/grammar.js).
1. Regenerate parser artifacts:

```sh
just generate
```

6. Run the local harnesses:

```sh
just test-grammar
INSTA_UPDATE=always just test-rust
```

7. Update docs if support status changed:

- [`README.md`](README.md)
- `CONTRIBUTORS.md`
- this [`AGENTS.md`](AGENTS.md) if the workflow itself changed

8. Run `hk fix` as the final mutating step.
1. Review snapshot changes carefully.
1. Run the upstream audit again for the narrow slice first, then more broadly:

```sh
STRUCTURIZR_UPSTREAM_FILTER=<slice> just audit-upstream
just audit-upstream
```

## How to decide where a test belongs

Use [`crates/strz-grammar/test/corpus/`](crates/strz-grammar/test/corpus/) when:

- the example is small
- the tree shape is important
- the syntax belongs in the stable Tree-sitter-native regression suite

Use [`fixtures/`](fixtures/) when:

- the example is more realistic or multi-block
- you want snapshot coverage for a representative DSL file
- you want the expectation encoded in the fixture filename (`-ok.dsl` / `-err.dsl`)

Use direct Rust tests only when:

- the behavior is really about the harness, not a DSL sample
- the assertion is awkward to express as a fixture file

If an `-err.dsl` fixture starts parsing cleanly because the grammar expanded, rename or rewrite it so the expected outcome remains intentional.

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
