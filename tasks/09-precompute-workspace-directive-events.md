## Issue

`process_document_context(...)` and sorts intermediary
directive data every time it processes a cache miss or invalidated context.

In include-heavy workspaces, that repeated directive-event rebuilding is a
plausible source of unnecessary load-time cost.

## Root Cause

In [`crates/strz-analysis/src/workspace.rs`](../crates/strz-analysis/src/workspace.rs),
`process_document_context(...)` clones a document's constant definitions and
include directives out of the snapshot and then feeds them through
`document_directive_events(...)`, which allocates and sorts a merged event
vector every time a context is processed.

At the same time, `DocumentContextKey::new(...)` clones the full inherited
constant environment into `Vec<(String, String)>` entries so the memoization key
stays ordered and comparable.

That keeps context processing deterministic, but it means repeated
include-heavy workspace loads for:

- cloning directive facts that are already stable in the snapshot
- re-sorting source-order directive events per processed context
- cloning every inherited constant binding into memoization keys

## Options

- Leave the current flow in place because it is straightforward and already correct.
- Precompute one stable source-order directive-event view per analyzed document and keep the current context-key shape.
- Precompute directive events and also replace the memoization key's cloned binding vector with a cheaper fingerprinted or interned representation.

## Proposed Option

Start by storing a stable directive-event view on the document snapshot so
workspace loading can iterate borrowed events in source order without per-call
cloning and sorting.

If context processing still dominates afterwards, follow with a second slice
that makes the inherited-constants memo key cheaper than cloning every binding
pair into a fresh ordered vector.
