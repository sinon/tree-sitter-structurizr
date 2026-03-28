## Issue

Local `just bench-rust` runs still put `analysis/document/large_big_bank_workspace` at roughly `1.07-1.09 ms`, so the steady-state document-analysis loop is now one of the clearer remaining hot paths.

## Root Cause

`crates/structurizr-analysis/src/parse.rs` parses once and then fans out into separate extraction passes for syntax diagnostics, includes, constants, identifier modes, and symbols/references.

The extractors in `crates/structurizr-analysis/src/extract/diagnostics.rs`, `extract/includes.rs`, `extract/constants.rs`, and `extract/symbols.rs` all recurse from the tree root again and eagerly allocate owned `String` values from node text. That keeps the implementation simple, but it means the document benchmark pays for several full-tree walks and repeated text extraction for every analysis run.

## Options

- Keep the multi-pass extractor layout because it is easy to read and maintain.
- Fuse the diagnostics/includes/constants/identifier-mode extractors into one preorder walk, while keeping the existing combined symbol/reference pass.
- Go further and build one unified extraction walker that emits every snapshot fact in a single traversal, possibly delaying `String` allocation until the final snapshot assembly.

## Proposed Option

Start with the middle path: merge the directive- and diagnostic-oriented passes first, keep symbol/reference extraction coherent, and only attempt a full single-pass extractor if benchmarks show there is still meaningful headroom.

That should trim the hottest repeated tree walks without making the extraction layer dramatically harder to reason about, and it preserves the current snapshot contract as the compatibility boundary.
