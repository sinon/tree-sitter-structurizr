## Issue

The local LSP benchmark still puts `lsp/session/large_big_bank_document_symbols` at roughly `9.31-9.39 ms`, and the current request flow rebuilds workspace facts on every open, change, and close before serving the follow-up request.

## Root Cause

`crates/structurizr-lsp/src/handlers/text_sync.rs` currently calls `recompute_workspace_facts(...)` from both `publish_latest_snapshot(...)` and `did_close(...)`.

That path clones the open-document set, reapplies overrides, creates a fresh `WorkspaceLoader`, and reloads the workspace from scratch for each buffer transition. The `documentSymbol` handler in `crates/structurizr-lsp/src/handlers/symbols.rs` itself is relatively small; the expensive part is the full rebuild and diagnostic republish path that happens before the request is answered.

## Options

- Keep full workspace recomputation for every open-buffer transition.
- Cache only canonical document identities and open-document lookup state, but still rebuild workspace facts from scratch.
- Add an incremental invalidation/update layer so one edited buffer can reuse most of the previously computed workspace state.

## Proposed Option

Take this in two slices.

First, cache canonical document identities and lookup state in the server so diagnostics and navigation helpers stop paying repeated filesystem normalization and linear open-document scans.

Then design an incremental workspace-facts update path for `didChange` / `didClose` so a single edited document does not force a whole-workspace reload when the workspace roots themselves have not changed.
