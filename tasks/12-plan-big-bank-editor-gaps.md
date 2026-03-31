## Issue

The remaining `# TODO:` markers in [`tests/lsp/workspaces/big-bank-plc/`](../tests/lsp/workspaces/big-bank-plc/) no longer point at one single "grammar gap."

They currently fall into three buckets:

- fixed-set style value completion in style settings such as `shape WebBrowser` and `shape MobileDeviceLandscape`;
- bounded view-reference coverage for multi-value `include` lists such as `include internetBankingSystem customer mainframe email` and identifier-valued `animation { ... }` steps;
- dynamic-view navigation for identifiers inside `dynamic` views.

Most of this surface already parses cleanly. The task is therefore mainly about query coverage, bounded reference extraction, and deliberate LSP scope decisions rather than basic parser recovery.

## Root Cause

- [`crates/structurizr-lsp/src/convert/completion.rs`](../crates/structurizr-lsp/src/convert/completion.rs) now offers style-property-name completion inside `element` and `relationship` style blocks, but it intentionally suppresses completions once the cursor moves into a style value. [`crates/structurizr-lsp/tests/navigation.rs`](../crates/structurizr-lsp/tests/navigation.rs) locks that behavior in today.
- [`crates/structurizr-grammar/grammar.js`](../crates/structurizr-grammar/grammar.js) allows repeated `field("value", $._view_value)` children inside `include_statement`, but [`crates/structurizr-analysis/src/extract/symbols.rs`](../crates/structurizr-analysis/src/extract/symbols.rs) currently reads only `node.child_by_field_name("value")`. That means only the first identifier in `include a b c` becomes a `ReferenceKind::ViewInclude`.
- [`crates/structurizr-grammar/grammar.js`](../crates/structurizr-grammar/grammar.js) also parses identifier-valued `animation` steps, but [`crates/structurizr-grammar/queries/highlights.scm`](../crates/structurizr-grammar/queries/highlights.scm) has no capture for animation values and the analysis crate does not yet emit bounded reference facts for them.
- Direct `dynamic_relationship` endpoints already parse and highlight, but the bounded-MVP docs in [`docs/lsp/02-design/`](../docs/lsp/02-design/) and [`docs/lsp/03-delivery/roadmap.md`](../docs/lsp/03-delivery/roadmap.md) explicitly defer dynamic-view relationship references. Broadening that support would change a documented boundary rather than just fill an isolated bug.

## Options

- Implement everything behind the remaining Big Bank TODOs in one umbrella task: style-value completion, multi-value `include`, `animation`, and `dynamic` view navigation.
- Split the work by subsystem and MVP boundary: keep style-value completion separate, extend the existing bounded view-reference slice to multi-value `include` and `animation`, and treat dynamic-view navigation as an explicit later phase.
- Keep the next task narrower and only fix multi-value `include` extraction for now, leaving `animation`, `dynamic`, and style-value completion to separate follow-ups.

## Proposed Option

Take the split option.

Style-value completion should stay separate from reference navigation.
It belongs with the existing style-completion work in [`tasks/06-plan-style-property-completion.md`](06-plan-style-property-completion.md), which already closed the property-name half of the problem.
Reopening that task for value completion would blur two different completion scopes.

The next bounded-navigation slice should focus on the lower-risk gaps that already fit the current model:

- extend `ViewInclude` extraction so every identifier-valued `include` argument is emitted, not just the first one;
- add query coverage and bounded reference extraction for identifier-valued `animation` steps;
- add targeted Big Bank regression tests that exercise `gotoDefinition` at those positions.

Dynamic-view relationship navigation should remain a separate option or follow-up phase unless the implementation deliberately updates the written MVP boundary in the same change.

That split keeps the task honest about current behavior, reuses the existing bounded-reference machinery, and avoids silently bundling a roadmap change into an otherwise routine gap-filling sweep.

## Resolution

Implemented the split option's bounded-navigation slice.

- `include` extraction now records every identifier-valued argument, including deployment-view include identifiers that resolve against deployment-layer bindings.
- The bounded analysis/LSP layer now records `ViewAnimation` references for supported static views and deployment views, with deployment animations resolving against deployment-layer bindings.
- The bounded analysis/LSP layer now also records `dynamic_view` scope identifiers plus explicit `dynamic_relationship` endpoint identifiers, so the Big Bank `dynamic apiApplication ...` subject and its `a -> b` endpoint identifiers navigate to their declarations.
- Query highlighting now treats identifier-valued `include` and `animation` positions as references.
- Added a dedicated low-level fixture plus snapshots for multi-value `include` and `animation`, and added a Big Bank `gotoDefinition` regression that covers a non-first `include` identifier, a static-view animation identifier, and a deployment-view animation identifier.

Style-value completion remains separate future work, and named dynamic-view relationship references remain deferred.
