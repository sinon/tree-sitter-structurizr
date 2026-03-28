## Issue

The local benchmark matrix still puts `analysis/workspace` at roughly `0.91 ms` / `1.20 ms` / `3.38 ms` for the small, medium, and large workspace cases, and the recent CodSpeed regression already showed that this path is sensitive to extra filesystem work.

## Root Cause

`crates/structurizr-analysis/src/workspace.rs` normalizes roots up front and then canonicalizes many paths again while scanning workspaces and following includes:

- `scan_workspace_root(...)`
- `resolve_local_include(...)`
- `collect_directory_include_paths(...)`

On the LSP side, `crates/structurizr-lsp/src/handlers/text_sync.rs`, `src/handlers/navigation.rs`, and `src/convert/diagnostics.rs` also call `fs::canonicalize(...)` when turning URIs back into workspace document identities. The same path normalization work is therefore repeated both within one workspace load and again on request/diagnostic paths.

## Options

- Keep canonicalization at every call site for simplicity.
- Add a per-load canonical-path cache in `structurizr-analysis` and store canonical path / `DocumentId` state alongside open documents in `structurizr-lsp`.
- Relax some normalization boundaries and canonicalize only selected edges.

## Proposed Option

Keep the current safety guarantees, but cache canonical forms aggressively so one discovered file or open document only pays for normalization once.

That means:

- a per-load cache for analysis-side path normalization and include traversal
- cached canonical path / `DocumentId` data in `DocumentState` or adjacent server state
- reusing that cached identity in diagnostics and navigation helpers instead of re-hitting the filesystem
