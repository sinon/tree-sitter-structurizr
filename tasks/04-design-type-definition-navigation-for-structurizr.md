## Issue

The copied Big Bank workspace expects a second navigation mode beyond ordinary definition: instances and deployment references should also support a "go to type definition" style jump to the underlying element type they represent.

The repository's current LSP docs do not define what `textDocument/typeDefinition` should mean for the Structurizr DSL.

## Root Cause

The current roadmap and bounded handler contract explicitly cover diagnostics, symbols, completion, definition, and references, but they do not mention a `typeDefinition` handler or equivalent semantics.

For Structurizr, type-style navigation is not trivial because the DSL mixes:

- the instance declaration itself
- the referenced model element that the instance represents
- deployment-node containment that may also feel like a navigable "type" to users

Without a design note, implementation would likely drift into ad hoc heuristics.

## Options

- Define a dedicated `textDocument/typeDefinition` feature for the DSL, with precise supported source sites and targets.
- Fold the behavior into `textDocument/definition` as an editor-specific heuristic and avoid a new protocol surface.
- Decide not to support type-definition semantics and document the reason clearly.

## Proposed Option

Write a short design note and roadmap amendment first. It should define what counts as a type-bearing site, what target each site should resolve to, and whether the feature should use the LSP's `typeDefinition` request or stay intentionally unsupported.

Only after that design exists should implementation be split into analysis and handler tasks.
