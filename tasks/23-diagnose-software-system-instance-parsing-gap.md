## Issue

The local grammar and fixture surface appear to mishandle
`softwareSystemInstance` in at least one standalone workspace fixture shape,
even though upstream Structurizr accepts the equivalent semantic intent.

This surfaced while making [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
upstream-compatible: the deployment example originally used
`softwareSystemInstance system`, but the local fixture parse failed and we had
to switch the example to `containerInstance system.app` instead.

That suggests a grammar/parity gap rather than an analysis-validation gap.

## Root Cause

[`crates/strz-grammar/grammar.js`](../crates/strz-grammar/grammar.js)
and its generated artifacts define the syntax accepted for deployment-instance
declarations.

The current fixture/test surface does not yet pin down whether
`softwareSystemInstance` is unsupported entirely, only unsupported in assigned
form, or blocked by a narrower ambiguity in the surrounding deployment-node
grammar.

Because the failure happened locally before semantic validation, the relevant
fix surface is grammar coverage and parser parity, not workspace diagnostics.

## Options

- Leave `softwareSystemInstance` coverage out of the standalone fixture surface
  and keep using container-instance examples only.
- Add a focused grammar/parity investigation that determines which
  `softwareSystemInstance` forms should parse, then fix the grammar and add
  coverage.
- Skip grammar changes and document the current limitation as an intentional
  deviation from upstream.

## Proposed Option

Run a focused grammar/parity investigation for `softwareSystemInstance`,
especially in deployment-node bodies with assigned identifiers.

If upstream accepts the shape, update the grammar and add both corpus and
fixture coverage; if upstream rejects the exact form, document the narrower
supported shape clearly and add a targeted negative test.

## Example future fixture

Suggested investigation fixture:
`fixtures/deployment/software_system_instance_assigned.dsl`

```dsl
workspace {
    model {
        system = softwareSystem "System"

        live = deploymentEnvironment "Live" {
            node = deploymentNode "Node" {
                systemInstance = softwareSystemInstance system
            }
        }
    }
}
```

Expected next step:

- determine whether this should be `-ok` or `-err` by comparing the local parse
  result with upstream behavior
