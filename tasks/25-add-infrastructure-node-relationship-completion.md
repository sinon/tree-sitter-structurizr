## Issue

The current identifier-completion slice does not help when an
`infrastructureNode` participates in a deployment-layer relationship.

That means the editor still returns no deployment-aware suggestions for the
widest deployment source family in the requested matrix: infrastructure nodes
should be able to target deployment nodes, infrastructure nodes, software
system instances, and container instances.

## Root Cause

[`crates/structurizr-lsp/src/convert/completion.rs`](../crates/structurizr-lsp/src/convert/completion.rs)
currently suppresses deployment-layer relationship completion altogether.

The analysis workspace index already exposes deployment bindings through
`unique_deployment_bindings()` and `duplicate_deployment_bindings()`, but the
LSP completion layer does not yet resolve deployment source kinds or filter
deployment destinations by that source-to-destination matrix.

## Options

- Keep infrastructure-node completion out of scope and land deployment-node-only
  completion first.
- Add infrastructure-node completion after deployment-node completion, with the
  requested destination matrix.
- Attempt a larger deployment validator/completion pass that also bundles other
  topology rules and diagnostics together.

## Proposed Option

Add infrastructure-node completion as the second deployment-layer completion
slice after deployment-node-only completion lands.

Source completion should include deployment bindings valid for an
`infrastructureNode`, and destination completion should filter to
`DeploymentNode`, `InfrastructureNode`, `SoftwareSystemInstance`, and
`ContainerInstance`. Coordinate the tests with
[`tasks/18-diagnose-deployment-parent-child-relationship-validation.md`](./18-diagnose-deployment-parent-child-relationship-validation.md)
so the deployment hierarchy rules and the completion matrix do not drift apart.
