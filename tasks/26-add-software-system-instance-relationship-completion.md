## Issue

The current completion slice still suppresses deployment-layer relationships
when the source endpoint is a `softwareSystemInstance`.

Per the requested deployment matrix, a software system instance should only
complete infrastructure-node destinations. That targeted rule is not yet
represented in the LSP completion layer.

## Root Cause

The shipped semantic completion path only resolves core element bindings and
intentionally returns no answer for deployment-layer relationship sites.

For `softwareSystemInstance` specifically, there is also an adjacent syntax/parity
risk already captured in
[`tasks/23-diagnose-software-system-instance-parsing-gap.md`](./23-diagnose-software-system-instance-parsing-gap.md).
The follow-up completion work should not assume every assigned instance form is
stable until that parser gap is settled.

## Options

- Defer software-system-instance completion until the parser/parity task is
  resolved.
- Land the completion slice now for the currently supported syntax shapes only.
- Fold software-system-instance completion into a larger instance-family
  deployment completion change.

## Proposed Option

Sequence this task after the parser/parity work in
[`tasks/23-diagnose-software-system-instance-parsing-gap.md`](./23-diagnose-software-system-instance-parsing-gap.md).

Once the supported syntax surface is pinned down, add deployment-aware source
kind resolution for `SoftwareSystemInstance` and filter destination completion
to `InfrastructureNode` only. Keep the multi-instance and duplicate-binding
suppression rules aligned with the core relationship-completion implementation.
