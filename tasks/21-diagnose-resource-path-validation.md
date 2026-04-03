## Issue

Our local analysis currently allows several file-backed directive and image-view
forms to stay "clean" even when the supporting filesystem resources or required
configuration are missing.

The recent fixture parity run found multiple upstream failures in this family:

- [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
  failed with `Documentation path /workspace/fixtures/views/docs does not exist`
- [`fixtures/workspace/directive_importers-ok.dsl`](../fixtures/workspace/directive_importers-ok.dsl)
  failed with `Documentation path /workspace/fixtures/workspace/docs does not exist`
- [`fixtures/views/advanced-ok.dsl`](../fixtures/views/advanced-ok.dsl)
  also failed with `Please define a view/viewset property named plantuml.url to specify your PlantUML server`

## Root Cause

[`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs)
already extracts path-bearing directives such as `!docs`, `!adrs`, and image
sources so editor features can find them.

[`crates/structurizr-analysis/src/workspace.rs`](../crates/structurizr-analysis/src/workspace.rs)
does not currently validate whether those referenced paths exist, whether they
contain the expected resource shape, or whether view properties such as
`plantuml.url` are present when a view form requires them.

That means local tooling can navigate these paths but still misses concrete
upstream resource-validation errors.

## Options

- Keep resource validation out of scope locally and rely on the upstream
  validator.
- Add bounded semantic diagnostics for missing docs/ADR/image resources and
  required supporting view properties.
- Build a broader filesystem-backed validator that models importer semantics and
  remote resource behavior in detail.

## Proposed Option

Add bounded semantic diagnostics for the concrete local cases we already expose
in editor features:

- missing local paths referenced by `!docs` / `!adrs`
- missing local source files for image-view directives
- missing required `views.properties` or `configuration.properties` entries
  such as `plantuml.url` when the chosen source mode requires them

That improves parity on contributor-owned files without taking on full importer
or remote-resource execution semantics.

## Example future `-err` fixture

Suggested fixture name: `fixtures/views/missing-plantuml-config-err.dsl`

```dsl
workspace {
    !docs "docs"

    views {
        image * "image-view" {
            plantuml "diagram.puml"
        }
    }
}
```

Expected upstream-style errors:

- `Documentation path /workspace/.../docs does not exist`
- `Please define a view/viewset property named plantuml.url to specify your PlantUML server`
