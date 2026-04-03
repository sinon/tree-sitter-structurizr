## Issue

Our local analysis is still too permissive about unresolved hierarchical
identifiers and `!element` selectors in extended workspaces.

The recent fixture parity run found two upstream failures in
[`fixtures/archetypes/find_element_paths-ok.dsl`](../fixtures/archetypes/find_element_paths-ok.dsl):

- `The destination element "route53" does not exist`
- `An element identified by "region" could not be found`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
extracts identifier references, selector-like targets, and path-like view
references for navigation.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently emit semantic diagnostics when a hierarchical identifier,
selector target, or inherited reference from an extended workspace cannot be
resolved to a concrete element in the assembled model.

This is especially visible when `!identifiers hierarchical` and `workspace
extends ...` combine: local tooling keeps enough structure for editor features,
but it does not yet reject the unresolved semantic reference.

## Options

- Keep hierarchical selector resolution permissive and rely on the upstream
  validator for unresolved-reference failures.
- Add a focused semantic diagnostic for unresolved `!element` selector targets
  and hierarchical references.
- Broaden the task into a general identifier-resolution validator across all DSL
  surfaces at once.

## Proposed Option

Add a focused semantic diagnostic for unresolved hierarchical identifiers and
`!element` selector targets in the assembled workspace, including inherited
identifiers that come from an extended base workspace.

That addresses the concrete upstream failures above and fits the current
bounded-analysis approach.

## Example future `-err` fixture

Suggested fixture name: `fixtures/archetypes/unresolved-hierarchical-selector-err.dsl`

```dsl
workspace extends "../deployment/aws-ok.dsl" {
    model {
        !element region {
            deploymentNode "Extra node" {
                infrastructureNode "Extra infrastructure node" {
                    -> route53
                }
            }
        }
    }
}
```

Expected upstream-style errors:

- `An element identified by "region" could not be found`
- `The destination element "route53" does not exist`
