## Issue

Our local analysis indexes view references without enforcing all upstream
element-compatibility rules for each view kind.

The recent fixture parity run found multiple upstream failures in this family:

- [`crates/structurizr-lsp/tests/fixtures/identifiers/view-animations-ok.dsl`](../crates/structurizr-lsp/tests/fixtures/identifiers/view-animations-ok.dsl)
  failed with `The element "worker" can not be added to this type of view`
- [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
  failed with `The element "user" can not be added to this type of view`
- [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
  also failed with `System is already the scope of this view and cannot be added to it`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already extracts view scopes, include arguments, animation references, and
definition targets for navigation.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently validate whether the referenced element kinds are legal for
the declared view type, or whether a view redundantly includes the element that
already defines its scope.

That makes the local tooling strong at navigation but still too permissive
about view semantics.

## Options

- Leave view-kind compatibility entirely to the upstream validator.
- Add bounded semantic diagnostics for unsupported element kinds and
  redundant-scope includes.
- Design a much broader view validator that tries to mirror most upstream view
  semantics in one pass.

## Proposed Option

Add bounded semantic diagnostics keyed by view kind and scope:

- reject element kinds that upstream does not allow in that view type
- reject explicit includes of the element that already defines the view scope
- apply the same compatibility checks to animation steps when they reuse the
  view's include surface

That turns the concrete upstream failures above into targeted local diagnostics
without forcing a one-shot reimplementation of all view semantics.
