## Issue

`strz check` can currently report a workspace as clean even when bounded
semantic analysis has enough information to know that identifier resolution
failed. The benchmark-mega parity work exposed this with
`!identifiers hierarchical`: upstream rejected flat nested identifiers, while
the local CLI still reported no diagnostics.

## Root Cause

[`crates/structurizr-analysis/src/diagnostics.rs`](../crates/structurizr-analysis/src/diagnostics.rs)
and
[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
already produce `SemanticDiagnostic`s for unresolved and ambiguous references.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
only renders syntax diagnostics and include diagnostics. It never includes
workspace semantic diagnostics, so unresolved identifier references do not fail
the main contributor-facing validation command.

That means reference mistakes that our analysis layer can already see remain
effectively invisible unless a contributor is looking through LSP diagnostics or
lower-level dump output.

## Options

- Keep `strz check` limited to syntax and include diagnostics and rely on editor
  diagnostics or upstream validation for semantic problems.
- Add an opt-in flag for semantic diagnostics while keeping the current default
  behavior.
- Make `strz check` include semantic diagnostics by default, alongside syntax
  and include diagnostics.

## Proposed Option

Teach `strz check` to surface workspace semantic diagnostics by default. That
keeps the CLI aligned with the LSP diagnostic surface and closes a major
leniency gap for already-supported semantic checks such as unresolved
hierarchical identifier references.

If compatibility concerns need a transition period, add a temporary escape hatch
instead of keeping semantic diagnostics hidden indefinitely.
