## Issue

The LSP integration tests still hide many request positions behind
`position_in(..., needle, byte_offset)` calls that force reviewers to count
bytes mentally.

This is already visible in the new hover tests, but it is broader than hover:

- [`crates/strz-lsp/tests/hover.rs`](../crates/strz-lsp/tests/hover.rs)
  currently uses the pattern for all hover request sites
- [`crates/strz-lsp/tests/navigation.rs`](../crates/strz-lsp/tests/navigation.rs)
  still has many definition/reference/path-navigation cases using magic offsets

## Root Cause

[`crates/strz-lsp/tests/support/mod.rs`](../crates/strz-lsp/tests/support/mod.rs)
currently exposes `position_in(text, needle, byte_offset_within_needle)`, which
is easy to implement but not easy to review.

That means test authors have to encode cursor intent indirectly, and reviewers
have to verify offsets by counting characters inside substrings rather than by
reading the fixture text directly.

## Options

- Keep the broader LSP suite on `position_in(...)` and only use marker-based
  positions for the hover cleanup.
- Add marker-based helpers now and migrate the highest-value navigation and
  hover tests incrementally.
- Do one immediate repo-wide migration of all LSP tests away from
  `position_in(...)`.

## Proposed Option

Add reusable marker-based helpers such as `<CURSOR>` and `<CURSOR:name>` in the
test support layer, migrate hover tests first, and then incrementally migrate
the existing navigation/reference tests that benefit most from visible cursor
intent.

That keeps the first cleanup focused while ensuring the broader readability win
is tracked explicitly instead of being forgotten.
