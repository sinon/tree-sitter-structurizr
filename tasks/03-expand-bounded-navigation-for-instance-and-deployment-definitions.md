## Issue

The Big Bank workspace expects definition-style navigation from deployment and instance references such as `containerInstance`, `softwareSystemInstance`, and deployment relationships to the bound declaration they represent.

This should be tracked separately from any future `typeDefinition` work. The missing piece here is only the ordinary definition side of those TODOs.

## Root Cause

The current bounded handler contract only supports definition and references for these extracted reference kinds:

- `RelationshipSource`
- `RelationshipDestination`
- `ViewScope`
- `ViewInclude`

That contract is documented in `docs/lsp/02-design/bounded-mvp-handlers.md`, and the current analysis extraction in `crates/structurizr-analysis/src/extract/symbols.rs` does not yet produce deployment- or instance-specific reference facts for the definition handler to consume.

## Options

- Extend the analysis layer with explicit deployment and instance reference kinds, then teach the existing definition/references handlers to resolve them conservatively.
- Reuse one of the existing reference kinds for deployment and instance sites to avoid schema growth, at the cost of muddier semantics.
- Defer the work until scope rules are expanded further, keeping the current handler surface unchanged.

## Proposed Option

Add explicit analysis facts for the deployment and instance sites that should participate in ordinary definition navigation, then extend `textDocument/definition` and `textDocument/references` only for those clearly supported shapes.

Anchor the work to the roadmap's later broader-reference-coverage expansion, and keep `typeDefinition` behavior out of scope for this task.
