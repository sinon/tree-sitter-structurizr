## Issue

The LSP integration tests now have marker-based cursor helpers, but
[`crates/strz-lsp/tests/navigation.rs`](../crates/strz-lsp/tests/navigation.rs)
still hides many request positions behind `position_in(..., needle, byte_offset)` calls that force reviewers to count bytes mentally.

The remaining work is narrower than this task originally described:

- [`crates/strz-lsp/tests/hover.rs`](../crates/strz-lsp/tests/hover.rs)
  already uses `<CURSOR>` / `<CURSOR:name>` markers
- [`crates/strz-lsp/tests/rename.rs`](../crates/strz-lsp/tests/rename.rs)
  also uses marker-based request sites
- [`crates/strz-lsp/tests/navigation.rs`](../crates/strz-lsp/tests/navigation.rs)
  still has the remaining magic-offset cases

## Root Cause

[`crates/strz-lsp/tests/support/mod.rs`](../crates/strz-lsp/tests/support/mod.rs)
already exposes marker-based helpers via `annotated_source(...)`, but it still
also exposes `position_in(text, needle, byte_offset_within_needle)`.

That leaves the navigation suite on the older helper even though the readable
marker path already exists and is used elsewhere.

## Options

- Keep the mixed helper model and accept the remaining `navigation.rs`
  readability cost.
- Migrate the remaining `navigation.rs` call sites incrementally now that the
  marker helper already exists.
- Do one immediate repo-wide cleanup and remove `position_in(...)` as soon as
  the last callers are migrated.

## Proposed Option

Finish the incremental migration by moving the remaining `navigation.rs`
definition/reference/document-link cases onto `<CURSOR>` / `<CURSOR:name>`
markers, then drop `position_in(...)` once it has no production callers left.

That matches the current codebase better than planning around the helper work
or the hover migration, both of which are already done.
