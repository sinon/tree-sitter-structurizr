# Structurizr DSL LSP docs

This directory documents the in-repo Structurizr analysis layer and language server: the durable design direction, the current implementation state, and the remaining delivery work.

If you only read one file first, read [`00-current-state.md`](./00-current-state.md).

## What these docs are for now

Use this directory when you need to:

- understand the architecture well enough to contribute safely
- see what is already shipped versus still intentionally bounded or deferred
- understand the remaining path to a feature-complete editor experience
- separate durable design decisions from planning-era or historical material

## Start here

1. [`00-current-state.md`](./00-current-state.md)
1. [`../../README.md`](../../README.md)
1. [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md)

## Directory layout

### [`01-foundations/`](./01-foundations/)

Durable architecture and boundary-setting docs.

Read these when you need to re-orient on:

- the editor-oriented goals and non-goals
- what belongs in the grammar versus the analysis layer versus the LSP layer
- how the grammar repo and downstream editor integrations fit together
- what should remain query-owned instead of being pushed through the LSP

### [`02-design/`](./02-design/)

Detailed contracts for the analysis layer, workspace model, scope rules, and handler boundaries.

Many of these docs describe shapes that are now implemented in code. They remain useful because they explain why the current implementation is intentionally conservative in some areas.

### [`03-delivery/`](./03-delivery/)

Integration, packaging, and sequencing docs.

These are the right place to look when you care about what remains before the current in-repo implementation feels feature-complete for downstream editor users.

### [`90-history/`](./90-history/)

Retained planning inputs and parser-shape audits from the early design pass.

Keep these for evidence and historical reasoning trails, but do not treat them as the primary starting point for current work.

## Suggested reading paths

### Contributor path

1. [`00-current-state.md`](./00-current-state.md)
1. [`01-foundations/overview.md`](./01-foundations/overview.md)
1. [`01-foundations/capability-matrix.md`](./01-foundations/capability-matrix.md)
1. [`03-delivery/roadmap.md`](./03-delivery/roadmap.md)
1. the specific design note in [`02-design/`](./02-design/) for the slice you are changing

### LSP user or integrator path

1. [`00-current-state.md`](./00-current-state.md)
1. [`03-delivery/roadmap.md`](./03-delivery/roadmap.md)
1. [`03-delivery/packaging-and-dev-loop.md`](./03-delivery/packaging-and-dev-loop.md)
1. [`03-delivery/zed-extension-language-server-wiring.md`](./03-delivery/zed-extension-language-server-wiring.md)

### Grammar/query consumer path

1. [`../../README.md`](../../README.md)
1. [`01-foundations/overview.md`](./01-foundations/overview.md)
1. [`01-foundations/query-ownership.md`](./01-foundations/query-ownership.md)

### Maintainer/reviewer path

1. [`00-current-state.md`](./00-current-state.md)
1. [`03-delivery/roadmap.md`](./03-delivery/roadmap.md)
1. [`01-foundations/repository-topology.md`](./01-foundations/repository-topology.md)
1. [`02-design/`](./02-design/)
1. [`90-history/`](./90-history/), only when you need the older reasoning trail

## Practical rule of thumb

If a document talks about already-shipped bounded behavior, read it as an implementation contract and rationale document.

If a document talks about downstream wiring, packaging, or broader semantic coverage, read it as active delivery or forward-looking design work.

If a document lives under [`90-history/`](./90-history/), treat it as historical context rather than a current entry point.
