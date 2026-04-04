## Issue

Typing the first identifier of a new relationship immediately before a following
`deploymentEnvironment` can produce cascaded `syntax.error-node` diagnostics in
the LSP.

`tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl` is a concrete
repro: inserting a partial source line before the `deploymentEnvironment`
statement near line 29 currently misparses enough of the remaining block to
surface noisy syntax diagnostics while the user is still mid-edit.

## Root Cause

The grammar currently interprets a lone identifier at that position as the start
of an `archetype_instance`, then greedily consumes the following
`deploymentEnvironment` line as the instance name and metadata.

That leaves the rest of the deployment block in recovery mode, and the LSP
publishes those transient syntax diagnostics directly from the broken parse.

## Options

- Leave the transient diagnostics as-is and rely only on completion to help the
  user finish the relationship quickly.
- Improve grammar recovery for incomplete relationship starts before following
  model items such as `deploymentEnvironment`.
- Keep the grammar unchanged, but suppress or soften the specific transient
  diagnostics in the LSP while the user is typing an incomplete relationship.

## Proposed Option

Investigate this as a focused parser/LSP recovery task with a regression fixture
based on the big-bank boundary.

Start by confirming whether the safer fix is grammar-side recovery for
incomplete relationship lines or a bounded LSP-side diagnostic suppression rule.
Whichever path is chosen should preserve the new relationship-source completion
behavior without hiding unrelated real syntax errors elsewhere in the file.
