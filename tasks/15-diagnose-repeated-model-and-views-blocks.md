## Issue

Our local workspace validation currently allows assembled workspaces that
upstream Structurizr rejects because they contain multiple top-level
`model { ... }` or `views { ... }` containers. The benchmark-mega corpus hit
this directly when upstream `validate` reported
`Multiple models are not permitted in a DSL definition`.

## Root Cause

The repository deliberately supports checked-in standalone `model { ... }` and
`views { ... }` fragments such as
[`tests/lsp/workspaces/minimal-scan/model.dsl`](../tests/lsp/workspaces/minimal-scan/model.dsl)
and
[`tests/lsp/workspaces/minimal-scan/views.dsl`](../tests/lsp/workspaces/minimal-scan/views.dsl),
because editor tooling needs to parse those files in isolation.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
currently assembles included documents additively and does not emit a semantic
diagnostic when one workspace instance ends up with more than one top-level
`model` block or more than one top-level `views` block.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
also has no dedicated parity rule for this assembled-workspace cardinality
constraint.

## Options

- Keep local tooling permissive here and rely on the upstream validator task to
  catch duplicate top-level containers.
- Add bounded semantic diagnostics for repeated top-level `model` and `views`
  containers within one assembled workspace instance.
- Stop treating wrapped fragments as valid local files and require only bare
  fragments.

## Proposed Option

Keep fragment parsing permissive, but add assembled-workspace semantic
diagnostics for repeated top-level `model` and `views` containers. That
preserves editor-facing fragment support while still telling contributors when a
whole workspace would be rejected upstream.
