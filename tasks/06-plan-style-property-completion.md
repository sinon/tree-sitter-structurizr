## Issue

`styles.element` blocks in the copied Big Bank workspace would benefit from completion of known property names such as `background`, `color`, and related style settings.

This is outside the current fixed-vocabulary completion MVP.

## Root Cause

The current completion implementation in [`crates/structurizr-lsp/src/convert/completion.rs`](../crates/structurizr-lsp/src/convert/completion.rs) only offers a static set of keywords and directives such as `workspace`, `model`, `!include`, and `autoLayout`.

The roadmap and bounded handler note deliberately stop short of context-aware property completion, and there is not yet a dedicated table describing which style-setting names should be offered in which block contexts.

## Options

- Treat style-property completion as a narrow, context-aware completion feature backed by a static DSL table, separate from general semantic identifier completion.
- Fold it into a later broader completion effort once scope rules and semantic completion are stronger.
- Leave it to editor snippets and avoid LSP completion for style properties.

## Proposed Option

Plan this as a narrow roadmap expansion rather than tying it to full semantic identifier completion. A static, context-aware completion table for style blocks could deliver useful editor value without waiting for general scope-aware completion.

If adopted, the follow-up implementation task should first define the supported property names and activation contexts before touching the completion handler.

## Resolution

Implement the narrow syntax-backed option.

- Keep semantic identifier completion separate from syntax-backed style-property completion.
- Add block-aware property-name completion for `element_style` and `relationship_style` bodies.
- Continue treating `!identifiers` and scope rules as concerns for identifier completion, not style-property completion.

Because the grammar still permits generic identifier-based style keys, this completion should be additive editor guidance rather than a validity gate.

Property-name completion is now implemented; broader value completion remains future work.
