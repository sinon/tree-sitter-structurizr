## Issue

The LSP `WorkspaceFacts` for the whole workspace on every
`didOpen` / `didChange` / `didClose`, even though it already has reusable loader
state and cached document identity.

That keeps edit-triggered request latency tied to whole-workspace recomputation
instead of the subset of workspace state actually affected by the change.

## Root Cause

[`crates/strz-lsp/src/server.rs`](../crates/strz-lsp/src/server.rs)
hosts one shared `WorkspaceLoader`, and
[`crates/strz-lsp/src/documents.rs`](../crates/strz-lsp/src/documents.rs)
already caches canonical path / `DocumentId` state for open documents.

[`crates/strz-lsp/src/handlers/text_sync.rs`](../crates/strz-lsp/src/handlers/text_sync.rs)
can also reuse a snapshot from freshly recomputed `WorkspaceFacts` instead of
immediately reanalyzing the current file again.

But `recompute_workspace_facts(...)` the open-document set,
clears/reapplies all overrides, and calls `load_paths(...)` for the whole
workspace on every buffer transition.

## Options

- Keep full workspace recomputation for every open-buffer transition.
- Add a coarser invalidation layer around the current shared loader/caches so
  only affected workspace roots or derived packets recompute.
- Push deeper workspace semantics behind another query boundary if coarse
  invalidation edited-root sessions too expensive.

## Proposed Option

Start by updating only the affected workspace roots or derived packets on
`didChange` / `didClose`, then re-measure.

If edited-root benchmarks still miss the target afterwards, follow with a deeper
workspace query boundary rather than another shallow host-side cache.
