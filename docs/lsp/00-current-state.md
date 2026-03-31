# Structurizr DSL LSP current state

This is the best starting point if you want to understand the design, what already works in-repo, and what still remains before the editor tooling feels feature-complete.

## Design direction

The project stays editor-oriented rather than runtime-oriented:

- the Tree-sitter grammar owns syntax, parse-tree shape, and portable query files
- `structurizr-analysis` owns extracted document facts and workspace/include modeling
- `structurizr-lsp` owns protocol handling and editor-facing request flow
- `structurizr-cli` exposes the same analysis and `strz server` entrypoints outside an editor loop
- downstream editor integrations such as `zed-structurizr` stay thin launchers and packaging layers

This layering is deliberate. We want good editor support for real `.dsl` files without turning this repository into an unofficial Structurizr runtime.

## Where we are now

The repository already contains:

- the checked-in grammar, parser artifacts, bindings, and query files
- [`crates/structurizr-analysis/`](../../crates/structurizr-analysis/) for document snapshots, symbol/reference extraction, diagnostics, and workspace discovery
- [`crates/structurizr-lsp/`](../../crates/structurizr-lsp/) for the stdio language server and handler layer
- [`crates/structurizr-cli/`](../../crates/structurizr-cli/) for `strz check`, `strz dump`, and `strz server`
- realistic multi-file fixtures and integration tests that exercise the bounded semantic surface

The question is no longer whether an in-repo bounded slice is feasible. That slice already exists. The useful question now is how to make the current editor-tooling stack easier to understand, easier to ship downstream, and broader without losing architectural discipline.

## Shipped today

Current in-repo behavior includes:

- syntax diagnostics from Tree-sitter parse errors
- include diagnostics for missing and cyclic file-resolution cases
- bounded semantic diagnostics for currently supported identifier families
- document symbols
- keyword/directive completion and style-property completion
- go-to-definition across the bounded symbol set, including cross-file cases already modeled in the workspace layer
- find-references across the same bounded symbol families
- type-definition for instance-to-model navigation
- document links for local directive paths, plus a definition fallback for editors that do not surface `textDocument/documentLink`

The important qualifier is still "bounded". The implementation already has real semantic value, but it stays conservative when the underlying scope model is not yet broad enough for a confident answer.

## Still intentionally bounded or deferred

The current implementation deliberately stays conservative around:

- `this`-based navigation and diagnostics beyond the cases already modeled safely
- selector and hierarchical reference forms such as `system.api`
- named dynamic relationship reference sites
- richer hover content
- identifier completion
- workspace symbols
- rename and code actions
- semantic tokens
- runtime-style validation or execution of `!script` / `!plugin`

Returning no answer for these cases is usually preferable to returning an answer that looks confident but is wrong.

## What "feature complete" means here

For this repository, "feature complete" does not mean upstream runtime parity. It means:

1. the downstream editor path is solid enough that users can reliably run the grammar and LSP in practice
2. the most important Structurizr reference shapes are navigable without surprising gaps
3. the read-only semantic UX feels whole enough to explain the model in-editor
4. edit-capable features only ship when the scope model is strong enough to make them safe
5. the workspace/indexing path is predictable enough to ship and maintain

## The main work still ahead

The remaining path to that state is roughly:

- finish downstream editor wiring and release choreography, especially around the separate Zed extension
- broaden safe reference coverage for selectors, `this`, named dynamic references, and other still-deferred scope cases
- add richer read-only features such as hover and workspace symbols
- add safe edit features such as rename only after broader reference coverage lands
- improve workspace invalidation, performance, and operational visibility so the current implementation scales more gracefully

## Suggested reading paths

### I want to contribute

1. [`../../README.md`](../../README.md)
2. [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md)
3. [`01-foundations/overview.md`](./01-foundations/overview.md)
4. [`01-foundations/capability-matrix.md`](./01-foundations/capability-matrix.md)
5. [`03-delivery/roadmap.md`](./03-delivery/roadmap.md)

### I want to use or integrate the LSP

1. [`../../README.md`](../../README.md)
2. [`03-delivery/roadmap.md`](./03-delivery/roadmap.md)
3. [`03-delivery/packaging-and-dev-loop.md`](./03-delivery/packaging-and-dev-loop.md)
4. [`03-delivery/zed-extension-language-server-wiring.md`](./03-delivery/zed-extension-language-server-wiring.md)

### I want to use only the grammar and queries

1. [`../../README.md`](../../README.md)
2. [`01-foundations/overview.md`](./01-foundations/overview.md)
3. [`01-foundations/query-ownership.md`](./01-foundations/query-ownership.md)

### I want to understand why the architecture looks this way

1. [`01-foundations/overview.md`](./01-foundations/overview.md)
2. [`01-foundations/repository-topology.md`](./01-foundations/repository-topology.md)
3. [`01-foundations/query-ownership.md`](./01-foundations/query-ownership.md)
4. [`02-design/`](./02-design/)
