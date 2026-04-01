> Status after recent Salsa merge: partially landed. Narrow this task to the remaining normalization edges.

## Issue

This task still applies, but its original LSP-side scope is now too broad. Local reruns put `analysis/workspace/large_big_bank_plc` at roughly `3.22-3.30 ms`, and file-backed LSP documents now already cache canonical path and workspace document identity.

The remaining path-normalization cost is now concentrated in workspace loading and a few LSP helper edges rather than every diagnostics/navigation path.

## Current State

On the LSP side, [`crates/structurizr-lsp/src/documents.rs`](../crates/structurizr-lsp/src/documents.rs) now caches both:

- canonical filesystem path
- canonical `DocumentId` / workspace identity

That means the original task text overstates the remaining LSP work. [`crates/structurizr-lsp/src/convert/diagnostics.rs`](../crates/structurizr-lsp/src/convert/diagnostics.rs) and [`crates/structurizr-lsp/src/handlers/navigation.rs`](../crates/structurizr-lsp/src/handlers/navigation.rs) now mostly operate on cached `workspace_document_id()` data instead of calling `fs::canonicalize(...)` themselves.

The remaining repeated normalization still shows up in:

- [`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs) via root normalization, include resolution, and directory-include traversal
- [`crates/structurizr-lsp/src/handlers/text_sync.rs`](../crates/structurizr-lsp/src/handlers/text_sync.rs) when workspace roots are canonicalized during recomputation
- [`crates/structurizr-lsp/src/handlers/directive_paths.rs`](../crates/structurizr-lsp/src/handlers/directive_paths.rs) when mapping include targets back to filesystem paths

## Options

- Keep canonicalization at the remaining call sites because the behavior is correct and the cost may already be acceptable.
- Add a per-load or per-session canonical-path cache in `structurizr-analysis` and stop re-canonicalizing already-known workspace roots in `structurizr-lsp`.
- Push canonical document/root identity more explicitly into the host-to-analysis boundary so helpers never need to hit the filesystem again for already-open files.

## Proposed Option

Keep the current safety guarantees, but narrow this task to the call sites that still pay repeated normalization work.

The pragmatic slice now is:

- a per-load normalization cache inside workspace loading and include traversal
- reusing already-canonical workspace roots in `text_sync`
- removing redundant canonicalization from directive-path helpers where cached document identity is already available

Do not spend time reworking the diagnostics/navigation helpers that this task originally named; that part is largely already done.
