## Issue

The analysis layer resolves dynamic-view endpoints for navigation, but
local validation accepts dynamic-view steps whose source/destination pair
does not correspond to a declared model relationship, while upstream
Structurizr rejects them.

The benchmark-mega parity work found this when upstream `validate` rejected
generated dynamic-view steps that narrated interactions we had not actually
declared in the model.

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already extracts `dynamic_view` scope identifiers and `dynamic_relationship`
endpoints for navigation and reference indexing.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently add a semantic validation pass that checks whether each
dynamic-view edge corresponds to an existing declared relationship in the
assembled model.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
already emits semantic diagnostics, but the analysis layer has no rule for this
upstream-parity mismatch yet.

## Options

- Keep relying on the upstream validator task to catch dynamic-view
  relationship mismatches.
- Add a focused semantic diagnostic that checks dynamic-view edges against
  declared model relationships.
- Attempt a much broader dynamic-view validator that also models ordering,
  parallel blocks, and explicit relationship references in one sweep.

## Proposed Option

Add a bounded semantic diagnostic for dynamic-view relationship existence.
Reuse the existing identifier extraction and workspace reference tables to
resolve the dynamic-view endpoints, then report a diagnostic when no matching
declared relationship exists in the assembled model.

That keeps this task focused on validation, and it covers the concrete upstream
mismatch we hit without forcing a full sequence-semantics implementation.
