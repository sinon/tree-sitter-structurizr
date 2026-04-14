## Issue

Workspace loading and directive-path handling the same local
paths repeatedly during include traversal, workspace recomputation, and path
translation.

Open documents canonical identities, so the remaining work is to
extend that reuse deeper into analysis-side loading and the remaining helper
paths.

## Root Cause

[`crates/strz-lsp/src/documents.rs`](../crates/strz-lsp/src/documents.rs)
stores `canonical_path` and `workspace_document_id` on `DocumentState`, and the
main diagnostics/navigation paths reuse that cached identity.

The remaining repeated normalization lives mostly in
[`crates/strz-analysis/src/workspace.rs`](../crates/strz-analysis/src/workspace.rs):

- `scan_workspace_root(...)`
- `resolve_include(...)`
- `collect_directory_include_paths(...)`

On the LSP side,
[`crates/strz-lsp/src/handlers/text_sync.rs`](../crates/strz-lsp/src/handlers/text_sync.rs)
configured workspace roots, and
[`crates/strz-lsp/src/handlers/directive_paths.rs`](../crates/strz-lsp/src/handlers/directive_paths.rs)
snapshot locations and include targets on demand.

## Options

- Keep canonicalization at the remaining call sites and accept the repeated
  filesystem work.
- Add a per-load canonical-path cache in `strz-analysis` and reuse
  canonical workspace root identities in the LSP helper layer.
- Relax some normalization boundaries and canonicalize only selected edges.

## Proposed Option

Keep the current safety guarantees, but make canonicalization reuse explicit at
the remaining workspace/loading edges:

- add a per-load cache for analysis-side path normalization and include traversal
- reuse canonical workspace root identities during recomputation
- remove helper-side canonicalization only where cached identities already make
  the result trustworthy
