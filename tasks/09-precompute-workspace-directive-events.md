> Status after recent Salsa merge: still open.

## Issue

After the new host-side workspace caches and the first private Salsa document cache landed, the cache-miss workspace-loading path still allocates and sorts intermediary directive data on every processed context.

This is now a clearer remaining cost because the broader loader/session reuse work is already in place.

## Current State

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs) now reuses processed document contexts across loads when it can, but cache misses still do the same per-context directive preparation:

- clone a document's constant definitions out of the snapshot
- clone a document's include directives out of the snapshot
- merge those facts through `document_directive_events(...)`
- sort the merged vector back into source order

At the same time, `DocumentContextKey::new(...)` still clones the full inherited constant environment into ordered `Vec<(String, String)>` entries so the memoization key stays comparable and deterministic.

## Options

- Leave the current flow in place because the surrounding caches already remove many rebuilds.
- Precompute one stable source-order directive-event view per analyzed document and keep the current context-key shape.
- Precompute directive events and then follow with a cheaper fingerprinted or interned inherited-constants key if the context key still profiles hot.

## Proposed Option

This task still looks like a good no-regret workspace-loading optimization.

Store a stable source-ordered directive-event view once per analyzed document, ideally alongside the existing syntax-fact boundary, so `process_document_context(...)` can iterate borrowed events without cloning and sorting on each cache miss.

If workspace loading still profiles hot afterwards, follow with a second slice that makes the inherited-constants memo key cheaper than cloning every binding pair into a fresh ordered vector.
