## Issue

The current identifier-completion slice returns no deployment-aware suggestions
when a `containerInstance` appears as the source endpoint of a deployment-layer
relationship.

Per the requested deployment matrix, a container instance should only suggest
infrastructure-node destinations.

## Root Cause

[`crates/structurizr-lsp/src/convert/completion.rs`](../crates/structurizr-lsp/src/convert/completion.rs)
currently suppresses deployment-layer relationship sites before candidate
selection starts.

The workspace index already holds container-instance bindings inside the
deployment binding tables, but the completion layer does not yet resolve source
instance kinds or narrow destinations to infrastructure nodes for this source
family.

## Options

- Leave container-instance completion deferred until all deployment-layer
  families are tackled together.
- Add one focused completion slice for `containerInstance -> infrastructureNode`
  after the earlier deployment follow-ups land.
- Roll container-instance completion into a broader deployment-layer semantic
  expansion that also adds diagnostics and hover work.

## Proposed Option

Add one focused follow-up slice after the deployment-node and
infrastructure-node tasks land.

Reuse the deployment binding tables and the existing conservative completion
policy, but resolve `ContainerInstance` sources explicitly and filter
destination suggestions to `InfrastructureNode` only. Keep this task scoped to
completion rather than bundling it with broader deployment diagnostics.
