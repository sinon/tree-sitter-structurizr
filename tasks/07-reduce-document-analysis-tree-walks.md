> Status after recent Salsa merge: largely superseded. Keep only as a cold-path follow-up, not a top-level hot-path task.

## Issue

The original motivation for this task is stale. Local reruns now put `analysis/document/large_big_bank_workspace` at roughly `16.17-16.35 µs`, not the earlier `~1.07-1.09 ms`.

That benchmark now reuses one `DocumentAnalyzer` across iterations, and `DocumentAnalyzer` now holds a private Salsa-backed parsed-document cache in [`crates/structurizr-analysis/src/parse.rs`](../crates/structurizr-analysis/src/parse.rs). In practice, the checked-in benchmark is mostly measuring steady-state cache hits rather than cold or changed-source document analysis.

## Current State

The underlying multi-pass extractor layout still exists. [`crates/structurizr-analysis/src/snapshot.rs`](../crates/structurizr-analysis/src/snapshot.rs) still fans out into separate passes for:

- syntax diagnostics
- includes
- constants
- identifier modes
- symbols/references

The extractors in [`crates/structurizr-analysis/src/extract/diagnostics.rs`](../crates/structurizr-analysis/src/extract/diagnostics.rs), `extract/includes.rs`, `extract/constants.rs`, and [`extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs) still recurse from the tree root independently and eagerly allocate owned `String` values from node text.

## Options

- Close or defer the task because the measured hot paths have moved elsewhere.
- Add a cold-path or changed-source benchmark first, then optimize only if document extraction still matters materially.
- Still fuse directive-oriented passes now as a no-regret cleanup, even without a strong benchmark signal.

## Proposed Option

Do not treat this as a top-priority performance task anymore.

If document-analysis latency becomes important again, first add a benchmark that invalidates the source or recreates the analyzer so the measurement captures real extraction cost. From that better baseline, revisit a fused extractor that merges the diagnostic/include/constant/identifier-mode walks while keeping the symbol/reference pass coherent.
