## Issue

The analysis layer resolves dynamic-view endpoints for navigation, but local
validation still accepts dynamic-view steps whose source/destination pair does
not correspond to a declared model relationship, or only matches after dropping
important relationship detail such as technology.

Recent fixture parity runs hit two concrete upstream failures:

- [`crates/structurizr-lsp/tests/fixtures/identifiers/dynamic-views-ok.dsl`](../crates/structurizr-lsp/tests/fixtures/identifiers/dynamic-views-ok.dsl)
  failed with `A relationship between Web Application and Sign In Controller does not exist in model`
- [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
  failed with `A relationship between User and App with technology HTTPS does not exist in model`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already extracts `dynamic_view` scope identifiers and `dynamic_relationship`
endpoints for navigation and reference indexing.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently add a semantic validation pass that checks whether each
dynamic-view edge corresponds to an existing declared relationship in the
assembled model, including a compatible technology when the step spells one
out explicitly.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
already emits semantic diagnostics, but the analysis layer has no rule for this
upstream-parity mismatch yet.

## Options

- Keep relying on the upstream validator task to catch dynamic-view
  relationship mismatches.
- Add a focused semantic diagnostic that checks dynamic-view edges against
  declared model relationships, including technology-aware matching.
- Attempt a much broader dynamic-view validator that also models ordering,
  parallel blocks, and explicit relationship references in one sweep.

## Proposed Option

Add a bounded semantic diagnostic for dynamic-view relationship existence and
compatibility. Reuse the existing identifier extraction and workspace
reference tables to resolve the dynamic-view endpoints, then report a
diagnostic when no matching declared relationship exists in the assembled
model for the same source/destination pair and optional technology.

That keeps this task focused on validation, and it covers the concrete upstream
mismatch we hit without forcing a full sequence-semantics implementation.

## Example future `-err` fixture

Suggested fixture name: `fixtures/views/dynamic-relationship-mismatch-err.dsl`

```dsl
workspace {
    model {
        user = person "User"
        system = softwareSystem "System" {
            app = container "App"
        }
    }

    views {
        dynamic system "dynamic-view" {
            1: user -> system.app "Requests data" "HTTPS"
        }
    }
}
```

Expected upstream-style error:

`A relationship between User and App with technology HTTPS does not exist in model`
