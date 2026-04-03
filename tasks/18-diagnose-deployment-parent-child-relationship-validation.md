## Issue

Our local analysis accepts deployment relationships whose endpoints are in a
parent/child containment relationship, but upstream Structurizr rejects them.

The recent fixture parity run hit this in
[`crates/structurizr-lsp/tests/fixtures/deployment/deployment-navigation-ok.dsl`](../crates/structurizr-lsp/tests/fixtures/deployment/deployment-navigation-ok.dsl),
where upstream `validate` reported:

`Relationships cannot be added between parents and children`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already resolves deployment identifiers and relationship endpoints for
navigation.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently add a semantic validation pass that checks whether a
deployment relationship connects an ancestor node to one of its descendants.

That leaves the local toolchain able to parse and navigate the reference while
missing an upstream deployment-topology rule.

## Options

- Keep local tooling permissive and rely on the upstream validator to reject
  parent/child deployment relationships.
- Add one bounded semantic diagnostic for deployment relationships whose source
  and destination are in the same containment chain.
- Attempt a broader deployment validator that also models more instance and
  topology constraints at the same time.

## Proposed Option

Add one narrow semantic diagnostic for deployment relationships whose source
and destination have an ancestor/descendant relationship in the assembled
deployment tree.

That captures the concrete upstream mismatch we saw without committing this
task to a full deployment-semantics reimplementation.
