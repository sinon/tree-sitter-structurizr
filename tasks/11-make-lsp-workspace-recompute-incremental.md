> Status after recent Salsa merge: still open and likely the highest-impact remaining performance task.

## Issue

This task still applies, but the original writeup is partially stale. Local reruns now put `lsp/session/large_big_bank_document_symbols` at roughly `6.16-6.26 ms`, down substantially from the older `~9.31-9.39 ms`, because the recent merge landed persistent loader/session reuse and additional host-side caches.

Even after that improvement, the LSP still rebuilds workspace facts on every open, change, and close before serving the follow-up request.

## Current State

Recent performance work already changed the baseline materially:

- `Backend` now holds a reusable `WorkspaceLoader` instead of constructing a fresh loader per edit.
- `WorkspaceLoader` now owns a longer-lived internal session with document, processed-context, per-root derived-instance, and final-assembly caches.
- file-backed `DocumentState` values cache canonical path and workspace identity.

Those changes removed a large chunk of the stale work described in the original task, but they did not change the outer invalidation boundary.

## Remaining Root Cause

[`crates/structurizr-lsp/src/handlers/text_sync.rs`](../crates/structurizr-lsp/src/handlers/text_sync.rs) still calls `recompute_workspace_facts(...)` from both `publish_latest_snapshot(...)` and `did_close(...)`.

That path still:

- rebuilds load inputs from current workspace roots and open documents
- clears and reapplies document overrides on the loader
- calls `WorkspaceLoader::load_paths(...)`
- stores a fresh `WorkspaceFacts` packet
- republishes diagnostics for every open document

The persistent loader makes repeated runs cheaper, but the invalidation model is still effectively "recompute whole workspace facts for every buffer transition" rather than "recompute only affected workspace instances/documents".

## Options

- Keep full recomputation, accepting the new caches as good enough.
- Add a lighter coarse invalidation layer so one edit can reuse unaffected workspace slices while still rebuilding affected instances as a whole.
- Move the next invalidation boundary into `structurizr-analysis`, likely via an analysis-owned Salsa-backed workspace query or another semantics-aware derived packet boundary.

## Proposed Option

Keep this task open and retarget it around coarse incremental invalidation, not loader reuse.

The first milestone should be to stop rebuilding and republishing more than the affected workspace slice on `didChange` / `didClose`. If that still leaves the edited-root benchmark too high, the next meaningful experiment should be the first true analysis-owned workspace query rather than another coarse host-side cache layer.
