# Structurizr DSL LSP overview

This repository now already ships the grammar, analysis layer, LSP, and contributor CLI that make up the current Structurizr editor-tooling stack. The durable question is no longer whether that shape is feasible. It is why the layers are split the way they are, and what responsibilities should stay stable as the implementation grows.

## The core design goal

The project is editor-oriented rather than runtime-oriented.

That means the repository should be good at:

- parsing real `.dsl` files faithfully
- exposing syntax structure through Tree-sitter and queries
- extracting stable editor-facing facts from that syntax
- answering navigation, diagnostics, and other semantic editor features

without trying to become a general Structurizr execution environment.

## Current layers

The current in-repo architecture is:

- repo root grammar crate: syntax, parser artifacts, Rust bindings, and portable query files
- `crates/structurizr-analysis/`: owned document snapshots, extracted facts, and workspace/include modeling
- `crates/structurizr-lsp/`: protocol-facing server state and request handlers
- `crates/structurizr-cli/`: the `strz` binary for local checks, dumps, and `strz server`
- downstream editor integration such as `zed-structurizr`: grammar pinning, launcher behavior, and editor-specific packaging

## Why the layers stay separate

Keeping these layers distinct preserves the main architectural safety rails:

- the grammar remains independently buildable and reusable by non-LSP consumers
- the analysis layer can evolve without leaking protocol types into every semantic data structure
- the LSP stays thin enough to consume analysis facts instead of re-walking syntax trees ad hoc
- downstream extensions can stay mostly launcher and packaging layers instead of becoming semantic forks

In other words, Tree-sitter stays the syntax source of truth, `structurizr-analysis` owns extracted facts, and the LSP mostly translates those facts into editor behavior.

## What Tree-sitter should keep owning

The grammar and checked-in queries are already the right home for:

- incremental parsing
- syntax-error detection through `ERROR` and `MISSING` nodes
- stable node-kind structure for editor features and extractors
- highlighting, folding, and indentation queries
- the syntax layer that downstream editor integrations can consume directly

Keeping this work Tree-sitter-native avoids forcing the LSP to duplicate editor behavior that is already cheaper and more portable as grammar/query assets.

## What the analysis layer should keep owning

`structurizr-analysis` is the bridge between parse trees and editor semantics.

It is the right home for:

- immutable document snapshots and syntax diagnostics
- extracted symbol and reference facts
- `!include` and `!identifiers` facts
- workspace discovery and include-following behavior
- the workspace facts that back bounded diagnostics and navigation

The important rule is that these remain transport-agnostic. The analysis layer should not take on LSP request types or editor-specific response shaping.

## What the LSP layer should keep owning

`structurizr-lsp` is the protocol and orchestration layer.

It should own:

- server capabilities and request routing
- document-open/change/close flow
- translation between analysis facts and LSP payloads
- LSP-specific compatibility behavior, such as path-opening fallbacks for editors that do not surface `textDocument/documentLink`

It should avoid becoming a second semantic engine. If a handler needs new semantic understanding, the right fix is usually to extend `structurizr-analysis`, not to special-case the handler.

## What downstream editor integrations should keep owning

Downstream integrations such as the separate Zed extension should stay thin.

They should own:

- grammar pinning and editor metadata
- editor-specific query surfaces that do not belong in the portable grammar repo
- locating or downloading the `strz` binary
- launching `strz server`

They should not become the place where Structurizr semantics are reimplemented.

## Current feature boundary

The current in-repo implementation already ships meaningful semantic behavior, but it is intentionally conservative:

- shipped today: syntax diagnostics, include diagnostics, bounded semantic diagnostics, document symbols, bounded completion, go-to-definition, references, type-definition, and directive-path links
- still deferred or partial: selector-based references, `this`, named dynamic relationship references, hover, identifier completion, workspace symbols, rename, and code actions

This is not a sign that the architecture is incomplete. It is a deliberate policy: the server should return no answer when the scope model is not broad enough to return a trustworthy one.

## Data flow in practice

The current happy-path pipeline is:

1. Tree-sitter parses source text into a syntax tree.
2. `structurizr-analysis` turns that tree into owned document facts.
3. The workspace layer merges per-file facts into include-aware workspace facts.
4. `structurizr-lsp` turns those facts into diagnostics, navigation results, and completion items.
5. `strz server` exposes the same LSP entrypoint that downstream editors should run.

## Non-goals that should stay stable

The repository should not drift into:

- executing `!script` or `!plugin`
- reproducing every upstream runtime rule before shipping useful editor features
- moving highlighting, folding, or indentation out of the grammar/query layer for no semantic reason
- treating the LSP as a proxy for the full Structurizr runtime or hosted platform

## What to read next

- [`../00-current-state.md`](../00-current-state.md) for the present-tense summary
- [`capability-matrix.md`](./capability-matrix.md) for feature status by layer
- [`repository-topology.md`](./repository-topology.md) for repo-boundary decisions
- [`../03-delivery/roadmap.md`](../03-delivery/roadmap.md) for the remaining path to feature-complete editor support
