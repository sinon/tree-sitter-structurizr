# Structurizr DSL query ownership decision

> Status: current decision.
>
> The grammar repo already owns the portable query surface, and the in-repo LSP
> now exists. Read future-tense wording below as ongoing query-planning context,
> not as evidence that the server is still hypothetical.

This document records where query files should live between:

- this grammar repository: `/Users/rob/dev/tree-sitter-structurizr`
- the Zed extension repository: `/Users/rob/dev/zed-structurizr`

The goal is to avoid accidental duplication, conflicting changes, and “which repo should this go in?” churn during future LSP work.

## Current observed state

## Grammar repo

Today this repository owns:

- `crates/structurizr-grammar/queries/highlights.scm`
- `crates/structurizr-grammar/queries/folds.scm`
- `crates/structurizr-grammar/queries/indents.scm`

And the Rust bindings already have a portable export path for:

- `HIGHLIGHTS_QUERY`
- future `TAGS_QUERY`
- future `LOCALS_QUERY`
- future `INJECTIONS_QUERY`

Notably, the Rust crate currently has **no parallel export surface** for:

- `outline.scm`
- `brackets.scm`

That is a strong signal that `tags.scm` is the natural portable symbol/query surface to add here first.

## Zed extension repo

Today the extension owns:

- `languages/structurizr/brackets.scm`
- `languages/structurizr/outline.scm`
- `languages/structurizr/textobjects.scm`
- local copies of `highlights.scm`, `folds.scm`, and `indents.scm`

That means the extension already has editor-specific query surfaces that do not exist in this grammar repo.

## Shared-query drift

Current shared-query comparison between the two repos:

- `folds.scm` — identical
- `indents.scm` — identical
- `highlights.scm` — diverged

This is exactly the kind of drift this decision is meant to reduce.

## Decision table

| Query | Canonical home | Why | Relationship to the other repo |
| --- | --- | --- | --- |
| `tags.scm` | Grammar repo | Portable symbol extraction is useful for the future analysis crate, LSP, and non-Zed consumers. The Rust bindings are already prepared to expose `TAGS_QUERY`. | No extension copy is required initially. Revisit only if a specific editor flow needs it directly. |
| `outline.scm` | Zed extension repo | Zed has a dedicated outline query surface and the current outline structure is editor-UX-oriented. It is not part of the grammar crate’s existing portable query/export story. | The grammar repo should not own `outline.scm` by default. Revisit only if another consumer needs a portable outline query distinct from tags. |
| `brackets.scm` | Zed extension repo | Bracket matching and rainbow-bracket behavior are editor-facing concerns, and this query already exists as extension-owned behavior. There is no current Rust binding/export surface for it in the grammar crate. | Keep extension-owned unless another editor or consumer creates a real need for a portable shared version. |

## Decision details

## `tags.scm` belongs in the grammar repo

Why:

- it is the most portable missing query surface
- it directly helps the future Rust analysis crate and LSP
- it fits the current Rust binding/export model
- symbol tagging is not a Zed-only concern

Implication:

- `tags.scm` should be added under `crates/structurizr-grammar/queries/` in this repository when the symbol-extraction work begins
- the LSP/analyzer should consume it from the grammar crate rather than depending on the Zed extension repo

## `outline.scm` stays in the Zed extension

Why:

- Zed has a dedicated outline query mechanism
- the current extension already has an outline tuned for editor structure and naming
- outline behavior is more editor-UX-shaped than grammar-crate-shaped

Implication:

- future grammar/LSP work should not block on moving outline into this repo
- if another consumer later needs a portable outline query, decide then whether that should be a new shared query or a derived form of `tags.scm`

## `brackets.scm` stays in the Zed extension

Why:

- it is squarely in the editor-behavior layer
- the current extension already owns it
- the grammar crate does not currently model/export bracket queries as part of its reusable Rust surface

Implication:

- bracket query tuning should happen in `zed-structurizr`
- no move is needed for the future LSP

## Sync policy for shared queries

The current split suggests two categories.

## Category A: portable shared queries

These should be canonical in the grammar repo:

- `highlights.scm`
- `folds.scm`
- `indents.scm`
- future `tags.scm`

The extension may carry copies because Zed expects language query files inside the extension, but those copies should be treated as mirrors of the grammar repo unless an intentional extension-only delta is documented.

## Category B: editor-owned queries

These should stay canonical in the Zed extension:

- `brackets.scm`
- `outline.scm`
- `textobjects.scm`

These are allowed to evolve around Zed behavior without forcing the grammar crate to absorb editor-specific policy.

## Immediate follow-up from this decision

1. Treat `tags.scm` as a grammar-repo task, not an extension-repo task.
2. Keep `outline.scm` and `brackets.scm` in `zed-structurizr`.
3. Reconcile the current `highlights.scm` drift by either:
   - upstreaming the extra extension captures into the grammar repo, or
   - explicitly documenting them as temporary extension-only deltas.

## What this unblocks

This decision means the next phases can proceed with a clear split:

- the grammar repo can add `tags.scm` to support portable symbol extraction for the future LSP
- the Zed extension can keep owning outline/bracket behavior without waiting on grammar-crate changes
- the two repos can share a cleaner “portable queries versus editor queries” boundary

## Sources

- `crates/structurizr-grammar/bindings/rust/build.rs`
- `crates/structurizr-grammar/bindings/rust/lib.rs`
- `crates/structurizr-grammar/queries/`
- `/Users/rob/dev/zed-structurizr/languages/structurizr/`
- `docs/lsp/01-foundations/repository-topology.md`
