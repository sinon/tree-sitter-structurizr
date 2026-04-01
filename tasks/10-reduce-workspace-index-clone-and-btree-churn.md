> Status after recent Salsa merge: still open, but now more clearly a cold/invalidated-path optimization than a whole-stack emergency.

## Issue

The new persistent workspace session, processed-context cache, per-root derived-instance cache, and final assembly cache already avoid a lot of repeated work across unchanged loads.

Even so, the remaining workspace-index construction path still does substantial ordered-map, set, clone, and sort/dedup work whenever one root or instance really has to be rebuilt.

## Current State

Local reruns put `analysis/workspace/large_big_bank_plc` at roughly `3.22-3.30 ms`, which is better than the older numbers but still means cold loads and invalidated roots pay for the current index-builder shape.

The main clone-heavy loops are still in [`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs):

- `build_workspace_indexes(...)`
- `collect_instance_documents(...)`
- `build_binding_tables(...)`
- `build_reference_resolution_tables(...)`
- `build_workspace_facts_assembly(...)`

Those loops still build intermediate `BTreeMap`, `BTreeSet`, and `Vec` structures, clone `DocumentId`, `SymbolHandle`, `ReferenceHandle`, and `SemanticDiagnostic` values, and then sort/dedup again at later boundaries. The new caches make this happen less often, but they do not make the rebuild itself cheaper.

## Options

- Keep deterministic `BTree*` containers end-to-end and accept the clone-heavy rebuild path.
- Use hash-based or indexed accumulation internally, then sort once when materializing stable public outputs.
- Introduce cheaper document ordinals or other interned identities first, while keeping the observable ordered containers unchanged.

## Proposed Option

Keep this task open, but treat it as secondary to invalidation-boundary work in the LSP path.

When it is worth pursuing, focus on making the internal accumulation loops cheaper while preserving deterministic ordering at the `WorkspaceFacts` boundary:

- reduce obvious `DocumentId` and handle cloning in instance collection and binding construction
- avoid repeated sort/dedup passes when one final ordered materialization is enough
- keep hash/index-based accumulation internal and sort only where results become observable
