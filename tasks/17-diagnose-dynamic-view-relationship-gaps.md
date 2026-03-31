## Issue

Our local tooling currently accepts dynamic-view steps whose source/destination
pair does not correspond to a declared model relationship, while upstream
Structurizr rejects them. The benchmark-mega parity work found this when
upstream `validate` rejected generated dynamic-view steps that narrated
interactions we had not actually declared in the model.

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already extracts `dynamic_relationship` endpoints for navigation and reference
indexing.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently add a semantic validation pass that checks whether each
dynamic-view edge corresponds to an existing declared relationship in the
assembled model.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
therefore has no upstream-parity signal for this class of view error.

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

That would cover the concrete upstream mismatch we just hit without forcing a
full sequence-semantics implementation.
