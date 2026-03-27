# Structurizr DSL LSP docs

This directory holds the long-lived planning and design material for a future Structurizr DSL language server.

The layout is intentionally split so it is easier to tell:

- which docs are foundational context
- which docs are durable design contracts
- which docs are delivery/sequence guides
- which docs are retained historical inputs rather than the primary place to start

## Directory layout

### `01-foundations/`

Long-lived context and boundary-setting docs.

Read these when you need to re-orient yourself on:

- why the LSP is feasible
- what should stay Tree-sitter-native vs semantic
- how the repos fit together
- where query ownership belongs

### `02-design/`

Durable design contracts for the future analysis layer, workspace model, scope rules, handler boundaries, and crate shapes.

These are the docs that should stay useful even after implementation begins, because they describe the intended semantic model rather than only a one-time task list.

### `03-delivery/`

Action-oriented sequencing and integration docs.

These are still important while the server is being built, but they are more likely to evolve or become partially superseded by real code and release processes.

### `90-history/`

Retained background material from the early planning pass.

These docs are still useful as evidence and parser-shape reference, but they are no longer the primary entry point once the design contracts exist.

## Suggested reading order

### Start here

1. `01-foundations/overview.md`
2. `01-foundations/capability-matrix.md`
3. `01-foundations/repository-topology.md`
4. `01-foundations/query-ownership.md`
5. `03-delivery/roadmap.md`

### Core design sequence

6. `02-design/analysis-crate-skeleton.md`
7. `02-design/first-pass-symbol-extraction.md`
8. `02-design/scope-rules.md`
9. `02-design/workspace-discovery-includes.md`
10. `02-design/workspace-index.md`
11. `02-design/lsp-crate-skeleton.md`
12. `02-design/bounded-mvp-handlers.md`

### Delivery and integration sequence

13. `03-delivery/zed-extension-language-server-wiring.md`
14. `03-delivery/packaging-and-dev-loop.md`
15. `03-delivery/advanced-semantic-expansion.md`

## Which docs are foundational vs transient

### Keep as long-lived references

- everything in `01-foundations/`
- everything in `02-design/`

These are the docs most worth retaining even after substantial implementation lands.

### Keep while implementation is active

- `03-delivery/roadmap.md`
- `03-delivery/zed-extension-language-server-wiring.md`
- `03-delivery/packaging-and-dev-loop.md`
- `03-delivery/advanced-semantic-expansion.md`

These remain useful, but they are naturally more likely to evolve as real crates, binaries, and extension wiring appear.

### Retain as historical/reference inputs

- everything in `90-history/`

In particular:

- `90-history/phase1-backlog.md` is an execution artifact from the initial planning phase
- the syntax audits are still useful evidence for why later extraction/design docs took the shape they did

## Practical rule of thumb

If you are trying to understand the current intended architecture, prefer:

- `01-foundations/`
- then `02-design/`
- then `03-delivery/`

Only drop into `90-history/` when you need:

- parser-shape background
- early backlog context
- the reasoning trail behind a design choice
