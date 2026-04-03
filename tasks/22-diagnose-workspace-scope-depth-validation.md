## Issue

Our local analysis currently accepts workspaces whose declared configuration
scope is incompatible with the depth of the modeled elements, while upstream
Structurizr rejects them.

The recent fixture parity run hit this in
[`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl), where
upstream `validate` reported:

`Workspace is landscape scoped, but the software system named System has containers.`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
extracts configuration directives and model declarations independently.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently add a semantic validation pass that cross-checks
`configuration { scope ... }` against the deepest model element kinds present in
the assembled workspace.

That leaves the local toolchain unable to flag a workspace-level consistency
rule that upstream already enforces.

## Options

- Keep workspace-scope validation out of local tooling and rely on upstream
  validation.
- Add a bounded semantic diagnostic that checks declared scope against model
  depth.
- Expand this into a much broader workspace-configuration validator.

## Proposed Option

Add a bounded semantic diagnostic that maps the declared configuration scope to
the maximum allowed model depth, then reports when the assembled workspace
contains elements that exceed that depth.

That captures the concrete upstream failure above and fits the current
incremental path better than a broad configuration-semantics rewrite.
