## Issue

Editor testing in the copied Big Bank workspace found three highlighting bugs:

- `securityComponent` highlights the `component` substring like a keyword
- `customer` highlights the `custom` substring like a keyword
- unquoted `url https://...` values render inconsistently

These are existing grammar/query bugs, not missing LSP features.

## Root Cause

Highlighting is query-owned in `queries/highlights.scm`, and the current query matches bare keyword strings such as `"component"` and `"custom"` as `@keyword`.

At the grammar layer, `grammar.js` defines `identifier` but does not define a `word` rule, which weakens keyword-boundary handling for Tree-sitter and makes substring bleed more likely. Unquoted URL values are currently parsed through generic `_directive_value` and `bare_value` handling rather than a dedicated URL token shape.

## Options

- Add a `word` rule in `grammar.js`, regenerate parser artifacts, and then tighten `queries/highlights.scm` only where needed.
- Leave the grammar alone and try to solve the substring bleed entirely in `queries/highlights.scm`.
- Introduce URL-specific grammar or query handling for `url_statement` values if the generic `bare_value` shape remains visually inconsistent after the boundary fix.

## Proposed Option

Prefer the structural fix first: add a proper `word` rule, regenerate the grammar outputs, and then refine `queries/highlights.scm` with regression coverage for identifier-vs-keyword and unquoted URL cases.

That keeps the fix in the grammar/query layer where ownership already lives and avoids pushing syntax-highlighting concerns into the LSP.
