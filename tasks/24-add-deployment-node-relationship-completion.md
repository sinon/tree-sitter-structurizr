## Issue

The LSP now completes flat-mode core element identifiers for explicit model
relationships, but it still returns no identifier completions for deployment
node relationship endpoints.

That leaves `deploymentNode -> deploymentNode` editing without the same guided
identifier insertion now available for core model relationships.

## Root Cause

[`crates/strz-lsp/src/convert/completion.rs`](../crates/strz-lsp/src/convert/completion.rs)
currently suppresses deployment-layer `relationship` surfaces intentionally.

The shipped completion candidate builder only walks
`WorkspaceIndex::unique_element_bindings()`, which covers `person`,
`softwareSystem`, `container`, and `component` bindings. Deployment-node
completion needs to switch over to deployment binding tables and keep the same
multi-instance and flat-mode safety rules.

## Options

- Leave deployment node completion deferred and rely on navigation only.
- Add one narrow follow-up slice for `deploymentNode -> deploymentNode`
  completion.
- Attempt a full deployment-layer completion rollout in one change.

## Proposed Option

Add one narrow follow-up slice for explicit deployment-node relationship
endpoints.

Reuse the existing workspace-aware completion path, but source candidates and
destination candidates should come from deployment bindings and should be gated
to `DeploymentNode` only for this task. Keep infrastructure nodes and instance
families in separate follow-up tasks so the deployment rollout stays incremental
and reviewable.
