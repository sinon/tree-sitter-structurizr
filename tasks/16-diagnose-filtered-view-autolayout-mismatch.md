## Issue

Upstream Structurizr rejects filtered views whose base view enables automatic
layout, but our local validation does not surface this rule today. The
benchmark-mega parity pass only caught it after upstream Docker validation
failed on a filtered landscape derived from an `autoLayout` base.

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
extracts view declarations and identifier references, but the workspace layer
does not currently model filtered-view compatibility rules between a filtered
view and its source view.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
already surfaces semantic diagnostics, but the analysis layer does not
currently emit one for this upstream-only view constraint.

The current toolchain is stronger at syntax structure and navigation than at
view-level semantic validation.

## Options

- Leave this rule entirely to the upstream validator script.
- Add one narrow semantic diagnostic for filtered views whose source view has
  automatic layout enabled.
- Design a broad, upstream-like view semantics validator that covers many more
  view rules at once.

## Proposed Option

Add a narrow workspace semantic diagnostic for this specific parity rule.
Resolve the filtered view's base view, detect whether that base view enables
`autoLayout`, and report the incompatibility at the filtered view site.

That gives us a concrete upstream-parity win without taking on a full
view-semantics reimplementation in one step.
