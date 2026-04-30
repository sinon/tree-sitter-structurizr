# Structurizr DSL LSP roadmap

This roadmap is no longer about proving that an in-repo bounded MVP can exist. That slice already exists. The useful question now is what remains before the current grammar-plus-analysis-plus-LSP stack feels feature-complete for real editor use.

## Current baseline

The repository already contains:

- the checked-in Tree-sitter grammar, bindings, and query files
- [`crates/strz-analysis/`](../../../crates/strz-analysis/) for extracted document and workspace facts
- [`crates/strz-lsp/`](../../../crates/strz-lsp/) for the stdio language server
- [`crates/strz/`](../../../crates/strz/) for `strz check`, `strz dump`, and `strz server`
- multi-file fixtures and LSP integration tests that cover the current bounded semantic surface

## Status of the original roadmap phases

| Original phase                          | Status now               | Notes                                                                                                                 |
| --------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| Phase 0: boundaries and dev loop        | Done in repo             | The grammar/analysis/LSP split is established, and the local CLI loop exists.                                         |
| Phase 1: grammar/query hardening        | Done as ongoing baseline | The grammar and query surface are real and continue to harden through fixtures and audits.                            |
| Phase 2: analysis crate                 | Done in repo             | `strz-analysis` is the transport-agnostic semantic layer.                                                             |
| Phase 3: workspace indexing             | Done in bounded form     | Workspace discovery, include-following, and bounded workspace facts already exist.                                    |
| Phase 4: LSP crate and bounded handlers | Done in repo             | The current server already ships diagnostics, symbols, completion, navigation, and links within the bounded scope.    |
| Phase 5: downstream editor delivery     | Done downstream          | The downstream launch path now exists; remaining repo work is to keep status and packaging docs aligned with reality. |
| Phase 6: broader semantic expansion     | Current                  | This is now the active in-repo track for making the bounded model feel more complete in practice.                     |

## What "feature complete" means here

For this repository, feature complete does not mean runtime parity with the upstream Java implementation.

It means:

- the downstream editor path is reliable enough that the server is easy to consume outside local contributor workflows
- the major Structurizr reference shapes that users expect in editors are navigable without surprising blind spots
- read-only semantic UX such as hover and workspace-level discovery feels solid
- edit-capable features only ship when the reference model is broad enough to make them safe
- the workspace/indexing path is operationally predictable enough to ship and maintain

## Current work streams

### 1. Keep the delivery docs aligned with the shipped downstream path

The separate downstream launch path is no longer the missing milestone it once was.

That means:

- removing stale wording that still treats Zed wiring as upcoming work
- keeping packaging and wiring notes as reference material rather than the active milestone
- avoiding accidental drift between shipped behavior, tests, and the delivery docs

See:

- [`packaging-and-dev-loop.md`](./packaging-and-dev-loop.md)
- [`zed-extension-language-server-wiring.md`](./zed-extension-language-server-wiring.md)

### 2. Complete the deferred parts of the current semantic model

The current bounded implementation is real, but some of the highest-value reference shapes still return no answer by design.

The next semantic expansion work should stay focused on the existing symbol families first:

- selector and hierarchical reference forms such as `system.api`
- `this`
- named dynamic relationship reference sites
- any other still-deferred scope cases that block confident navigation for already-supported constructs

This is the main work that makes the current server feel less "bounded" without changing the architecture.

### 3. Round out the read-only semantic UX

Once the broader reference model is strong enough, the next high-value additions are read-only features that explain the model better in the editor.

That includes:

- richer hover
- workspace symbols
- broader identifier completion
- richer diagnostic messages where the current semantic model already has the underlying facts

These features were usually safer to add before the first bounded rename slice
because they expose information without rewriting source text.

### 4. Add edit-capable features only when they are safe

Broader rename work and code actions should stay later work.

They still depend on:

- broader reference coverage
- conflict checks
- stronger guarantees around scope and deferred cases

The guiding rule is simple: do not broaden edit-capable features until the
analysis layer can explain their answer with the same confidence as read-only
navigation.

### 5. Improve operability and performance

The current in-repo implementation is useful, but it still has room to become cheaper and more predictable.

The main follow-on work here is:

- cached workspace invalidation instead of whole-workspace recomputation on every relevant update
- better surfacing of filesystem and workspace-loading failures
- continued use of the existing benchmark harnesses against representative workspaces such as `big-bank-plc`

## What not to chase

The remaining roadmap should stay disciplined.

We should not treat "feature complete" as a reason to chase:

- `!script` or `!plugin` execution
- full runtime parity before shipping useful editor behavior
- moving query-native editor features into the LSP with no semantic benefit
- protocol polish that outruns the analysis model

## Companion docs

- [`../00-current-state.md`](../00-current-state.md) for the present-tense summary
- [`../01-foundations/overview.md`](../01-foundations/overview.md) for the durable architecture split
- [`../01-foundations/capability-matrix.md`](../01-foundations/capability-matrix.md) for feature status by layer
- [`advanced-semantic-expansion.md`](./advanced-semantic-expansion.md) for the longer-term post-bounded feature tracks
- [`../90-history/`](../90-history/) if you need the earlier planning trail rather than the current delivery view
