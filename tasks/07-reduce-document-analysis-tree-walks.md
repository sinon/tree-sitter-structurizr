## Issue

Fresh or invalidated document analysis the same parse tree multiple
times while assembling `DocumentSyntaxFacts`.

`DocumentAnalyzer` redoing work for unchanged documents, but the
cold-path extraction cost still matters for first analysis, changed files, and
any future workspace invalidation flow that has to rebuild document facts.

## Root Cause

[`crates/strz-analysis/src/parse.rs`](../crates/strz-analysis/src/parse.rs)
caches `parsed_document(...)` behind the Salsa-backed
`IncrementalAnalysisDatabase`.

But [`crates/strz-analysis/src/snapshot.rs`](../crates/strz-analysis/src/snapshot.rs)
still has `DocumentSyntaxFacts::collect(...)` fan out into separate passes for
syntax diagnostics, includes, constants, identifier modes, and
symbols/references.

Those extractors from the tree root independently and
allocate owned text eagerly from the same source.

## Options

- Treat the current multi-pass layout as good enough and rely on existing
  document caching.
- Fuse the diagnostics/includes/constants/identifier-mode extractors into one
  traversal while keeping symbol/reference extraction separate.
- Attempt a full single-pass extractor only if fresh cold-path benchmarks still
  show meaningful headroom.

## Proposed Option

If a fresh cold or invalidated-source benchmark still shows document analysis as
a meaningful cost, start with the middle option: merge the
directive/diagnostic-oriented passes first, re-measure, and only then consider a
fully unified extractor.
