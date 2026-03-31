# Structurizr DSL repo topology and integration options

> Status: implemented decision.
>
> The grammar, analysis crate, LSP crate, and separate Zed extension now exist
> in this shape. Read remaining future-tense wording below as the rationale for
> why that topology was chosen and why it should stay stable.

## Why this matters

There are already two moving pieces:

- this repository, which is both the Tree-sitter grammar source and the Rust parser crate
- `/Users/rob/dev/zed-structurizr`, which is the Zed extension repository

Adding an LSP creates a third moving piece.

That could become painful if grammar changes, LSP changes, and extension changes all need to land in lockstep.

## Current reality

Today the Zed extension:

- declares the grammar in `extensions.toml` via `[grammars.structurizr]`
- pins this grammar repository by git revision
- keeps Zed-side language config and editor query files in `languages/structurizr/`

That means the extension is consuming this repository as a **grammar repository**, not just as a Rust crate dependency.

This creates an important constraint:

- the grammar must remain buildable from a standard Tree-sitter grammar directory that Zed can target explicitly

That no longer means the grammar has to live at the repository root. Zed can pin
this repository and build the grammar from [`crates/structurizr-grammar/`](../../../crates/structurizr-grammar/) via the
grammar entry's `path` field, so the Cargo workspace can use a clearer internal
layout without making extension development awkward.

## Option A: keep grammar and LSP together, keep Zed extension separate

### Shape

- `tree-sitter-structurizr`
  - grammar source and generated artifacts
  - Rust bindings
  - future analysis crate
  - future LSP crate
- `zed-structurizr`
  - Zed extension manifest
  - Zed language config
  - Zed-specific query files
  - future language-server launch wiring

### Why this is the best near-term fit

- grammar and LSP are the two pieces most likely to evolve together quickly
- the Zed extension already has a clean downstream-consumer role
- this repository can stay a normal grammar repo for Zed while also hosting Rust crates for analysis and the server
- local development is much faster when grammar and LSP live in the same Cargo workspace

### Downsides

- releases still need coordination into the Zed extension
- this repo becomes broader than “just a grammar”
- there needs to be discipline around what is parser-owned versus extension-owned

### Recommendation

This should be the default plan unless later maintenance pain proves otherwise.

## Option B: three separate repos from the start

### Shape

- grammar repo
- LSP repo
- Zed extension repo

### Benefits

- very clean separation of responsibilities
- independent release cadence for each artifact
- easier to make the LSP editor-agnostic over time

### Costs

- highest coordination overhead during early development
- grammar fixes needed by the LSP require cross-repo updates immediately
- the extension has to track both a grammar revision and an LSP release
- local development becomes slower unless a lot of tooling is built around it

### Recommendation

This is attractive later if the LSP matures into its own product surface, but it is likely too expensive as the starting point.

## Option C: merge grammar, LSP, and Zed extension into one repo

### Shape

One repository contains:

- the Tree-sitter grammar at the root
- Rust bindings
- future analysis/LSP crates
- `extensions.toml`
- `languages/structurizr/`

### Why it is tempting

- a single commit can update grammar, LSP, and extension behavior together
- no repo-crossing pin churn during active development

### Risks

- the repository root would need to satisfy both “grammar repo” and “Zed extension repo” expectations at the same time
- Zed-specific files and grammar-internal files become tightly coupled
- release noise increases for consumers who only care about one layer
- future reuse by non-Zed consumers becomes less clean

### Recommendation

Possible, but only worth reconsidering if the current two-repo setup plus an in-repo LSP still creates too much coordination friction.

## Recommended direction

Near term:

- keep grammar + analysis + LSP in this repository
- keep the Zed extension separate

Longer term:

- revisit only after the MVP server exists and the maintenance pain is concrete

## How local development can stay fast

Zed's docs allow grammar repositories to be loaded from `file://` URLs during local extension development.

That gives a good workflow:

1. keep working on grammar and LSP in this repository
2. point the dev extension at `file:///Users/rob/dev/tree-sitter-structurizr` with `path = "crates/structurizr-grammar"` for grammar changes
3. point the extension at a locally built LSP binary for server changes
4. only pin commit SHAs and package binaries when preparing a real extension release

This keeps fast iteration local without forcing every experiment through git tags and published artifacts.

## Suggested responsibility split

### Grammar repo

Own:

- [`crates/structurizr-grammar/grammar.js`](../../../crates/structurizr-grammar/grammar.js)
- generated parser artifacts
- Rust bindings
- portable query surfaces that make sense outside Zed
- future analysis crate
- future LSP crate

### Zed extension repo

Own:

- `extensions.toml`
- `languages/structurizr/config.toml`
- Zed-specific query surfaces and editor tuning
- LSP launch wiring and packaging decisions

## Query ownership note

The current query surfaces are already split:

- this repository has `highlights.scm`, `folds.scm`, and `indents.scm`
- the Zed extension also has `brackets.scm`, `outline.scm`, and `textobjects.scm`

That split is not necessarily wrong, but it means future work should decide consciously:

- which queries are portable and should live with the grammar
- which queries are editor-specific and should remain in the Zed extension

The current recommended split is captured in [`docs/lsp/01-foundations/query-ownership.md`](query-ownership.md).

## Release-flow suggestion

For actual releases:

1. land grammar and LSP changes here
2. cut a grammar/LSP release or at least a stable commit
3. update the Zed extension's pinned grammar revision
4. update the Zed extension's LSP packaging or binary reference
5. smoke-test the extension against realistic `.dsl` files

This preserves loose coupling in releases without slowing down day-to-day development.

The concrete launcher and packaging recommendation for that flow is captured in [`docs/lsp/03-delivery/zed-extension-language-server-wiring.md`](../03-delivery/zed-extension-language-server-wiring.md).
