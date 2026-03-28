## Issue

The workspace-index construction path still does a lot of ordered-map, set, clone, and sort/dedup work in the hottest analysis loop, even after the tracing rollback.

## Root Cause

`crates/structurizr-analysis/src/workspace.rs` builds the final workspace facts through several clone-heavy phases:

- `cycle_include_indices(...)`
- `build_workspace_indexes(...)`
- `build_binding_tables(...)`
- `build_reference_resolution_tables(...)`
- `merge_semantic_diagnostics(...)`

Those loops currently clone `DocumentId`, `SymbolHandle`, and `SemanticDiagnostic` values into temporary `BTreeMap`, `BTreeSet`, and `Vec` structures, then sort and deduplicate again at later boundaries. Some of the reference-resolution helpers also do repeated map lookups for the same key before materializing a result.

## Options

- Keep deterministic `BTree*` containers end-to-end and accept the clone-heavy inner loops.
- Use hash-based or indexed accumulation internally, then sort once when materializing the stable public outputs.
- Introduce interned or numeric document ordinals first, while keeping the surrounding ordered containers unchanged.

## Proposed Option

Keep the externally visible ordering stable, but move the inner accumulation loops toward cheaper keyed structures and fewer repeated clones.

A pragmatic first slice would:

- remove obvious double-lookups in reference resolution
- reduce `DocumentId` / handle cloning inside cycle detection and reference indexing
- sort or deduplicate once at the boundary where `WorkspaceFacts` becomes observable
