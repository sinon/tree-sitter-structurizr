# Structurizr DSL LSP crate skeleton

> Status: implemented in bounded form.
>
> `structurizr-lsp` now exists in-repo. Read this note as the crate-shape
> contract and rationale for keeping the server thin rather than as a greenfield
> proposal.

This note turns Phase 4 of [`docs/lsp/03-delivery/roadmap.md`](../03-delivery/roadmap.md) into a concrete crate shape.

Its job is to define the first language-server crate that:

- depends on the future analysis crate rather than re-implementing semantic logic
- stays small enough to remain mostly protocol glue and cache orchestration
- is easy to wire into Zed and similar editors as a stdio-launched binary

The goal is not to design every later feature in detail.

The goal is to make the first server implementation start from stable boundaries, a stable state model, and a realistic MVP handler set.

## Why this crate needs to stay thin

The analysis crate is where Structurizr-specific semantic extraction should live.

That includes:

- syntax diagnostics as reusable facts
- extracted directives such as `!include` and `!identifiers`
- symbols and references for the bounded MVP
- later workspace-instance and include-resolution facts

The LSP crate should sit above that layer and primarily own:

- protocol transport
- open-document state
- request/notification routing
- conversion into LSP types
- publishing diagnostics and results to the client

If semantic extraction starts leaking back into request handlers, the LSP will become hard to test, hard to evolve, and too tightly coupled to one editor/runtime surface.

## Current repo constraints

This repository already has an important packaging shape:

- the parser crate package lives at the repository root
- the future analysis crate will be a nested Cargo workspace member
- the future LSP crate should live in the same workspace
- the repository root must remain a normal Tree-sitter grammar repo for Zed

That means the LSP crate should be added alongside the analysis crate:

```text
Cargo.toml                      existing parser crate package at repo root
crates/structurizr-analysis/    future analysis crate
crates/structurizr-lsp/         future LSP crate
```

The LSP crate should not force the grammar repo to stop looking like a standard Tree-sitter grammar repository.

## Recommended package shape

The LSP crate should be both:

- a small library that exposes the backend/state/handler construction points
- a binary that launches the server over stdio

That gives a better testing story than pushing everything into `main.rs`.

Recommended shape:

```text
crates/structurizr-lsp/
  Cargo.toml
  src/lib.rs
  src/main.rs
  src/server.rs
  src/state.rs
  src/documents.rs
  src/capabilities.rs
  src/config.rs
  src/convert/
    mod.rs
    diagnostics.rs
    positions.rs
    symbols.rs
    completion.rs
  src/handlers/
    mod.rs
    lifecycle.rs
    text_sync.rs
    diagnostics.rs
    symbols.rs
    completion.rs
    goto_definition.rs
    references.rs
  tests/
    lifecycle.rs
    diagnostics.rs
    navigation.rs
```

The exact filenames can change, but the role split should stay visible.

## Why this shape

- `main.rs` should be nearly empty and only bootstrap stdio transport
- `lib.rs` should expose testable construction points
- `server.rs` should define the `tower-lsp-server` backend type
- `state.rs` should own shared server state rather than scattering it across handlers
- `documents.rs` should isolate open-document tracking and text/version logic
- `capabilities.rs` should build advertised server capabilities in one place
- `convert/` should isolate conversion between analysis-layer facts and `lsp-types`
- `handlers/` should keep feature entrypoints separate instead of collapsing into one giant `impl LanguageServer`

## Recommended Cargo/dependency shape

The eventual workspace should include both future crates:

```toml
[workspace]
resolver = "2"
members = [
  "crates/structurizr-analysis",
  "crates/structurizr-lsp",
]
```

And the LSP crate should look roughly like:

```toml
[package]
name = "structurizr-lsp"
edition = "2021"

[dependencies]
structurizr-analysis = { path = "../structurizr-analysis" }
tower-lsp-server = "0.20"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std", "sync"] }
lsp-types = "0.97"
line-index = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
assert-json-diff = "2"
```

Important boundary:

- the LSP crate should depend on `structurizr-analysis`
- it should avoid a direct dependency on the parser crate unless a very specific transport concern requires it

That keeps the “semantic layer below protocol layer” boundary real.

## Transport choice

The first implementation should use:

- `tower-lsp-server`
- stdio transport
- a single binary launched by editors such as Zed

Why:

- it is the transport/tooling choice already recommended in the planning docs
- it keeps boilerplate low for a greenfield server
- it gives a clear `LanguageServer` implementation surface

This should remain the default unless later implementation proves a real need to drop lower-level.

The crate skeleton should therefore optimize for `tower-lsp-server` ergonomics first rather than pre-optimizing around hypothetical transport pain.

## Core design rules

### Rule 1: the LSP should read analysis facts, not raw trees

Handlers should operate on:

- `DocumentSnapshot`
- workspace-instance/index facts
- converted LSP-friendly views of those facts

They should **not** re-walk Tree-sitter trees in each request handler.

If a handler needs data the analysis crate does not expose yet, that is a sign to extend the analysis crate rather than bypass it.

### Rule 2: `main.rs` should only bootstrap transport

The binary entrypoint should do little more than:

- initialize logging later if needed
- create the backend/service
- connect stdin/stdout to `tower_lsp::Server`
- await the server future

That keeps the real logic importable from tests and future packaging code.

### Rule 3: keep protocol conversions in one place

The analysis crate should expose transport-agnostic facts.

The LSP crate should own:

- byte/point to LSP position conversion
- analysis diagnostics to `lsp_types::Diagnostic`
- symbol/reference facts to `Location`, `DocumentSymbol`, `CompletionItem`, etc.

That conversion logic should live under `convert/`, not scattered through handlers.

### Rule 4: do not advertise capabilities before the backing logic exists

The first server should advertise only what it can support reliably.

That means no early advertising of:

- rename
- hover
- workspace symbols
- semantic tokens
- code actions

until the corresponding analysis and workspace logic exists.

### Rule 5: keep async orchestration out of semantic types

Async belongs in transport/state/handler orchestration.

The analysis facts should stay synchronous and owned.

The LSP crate can then:

- schedule analysis work
- publish results
- discard stale results when newer document versions arrive

without polluting the semantic layer with async concerns.

## Recommended initial module responsibilities

### `server.rs`

Own:

- the backend type implementing `tower_lsp::LanguageServer`
- a constructor that receives a `tower_lsp::Client`
- thin delegation from protocol methods into handler modules

This file should not contain the whole server logic inline.

### `state.rs`

Own:

- client capabilities captured at initialization
- workspace roots from `workspaceFolders` / `rootUri`
- shared open-document state
- latest analysis snapshots keyed by document
- latest workspace-level facts/indexes
- server configuration state if any later config is added

Start simple.

A single shared state object with small lock scopes is better than prematurely splitting into many concurrent caches.

### `documents.rs`

Own:

- open document text
- document versions
- language IDs if useful
- file-backed versus in-memory identity details
- helper methods for applying open/change/close lifecycle events

This file is the right place to hide the exact text-storage strategy from handlers.

### `capabilities.rs`

Own:

- construction of `ServerCapabilities`
- the initial `TextDocumentSyncOptions`
- any later feature-gating based on client capability inspection

Keeping this in one place makes it easy to review what the server is promising.

### `handlers/`

Own one feature area per module:

- lifecycle
- text sync
- diagnostics
- symbols
- completion
- go-to-definition
- references

These handlers should read from state and call into conversion helpers, not become mini state-management systems themselves.

## Document sync strategy

The first LSP crate should bias toward a simple and reliable text model.

### Recommended MVP choice

Start with:

- `TextDocumentSyncKind::FULL`
- whole-document reanalysis on open/change
- `String`-backed text storage plus `line-index` for position conversion

Why this is the right first boundary:

- it matches the analysis crate's first snapshot-oriented API
- it avoids taking on incremental text patching before the semantic model is stable
- Structurizr DSL files are typically small enough that this is a reasonable MVP tradeoff

### Where `ropey` fits

`ropey` is still a good future building block.

But it should only become mandatory when one of these becomes true:

- incremental sync is implemented
- profiling shows full-string replacement is a problem
- later editor support makes richer edit handling necessary

That means the crate skeleton should hide text storage behind `documents.rs` rather than forcing `ropey` into day-one public expectations.

### Position conversion

`line-index` should still be used early.

The analysis layer will work with bytes/points.

The LSP layer needs:

- line/character ranges
- UTF-16-aware positions for protocol results

That is an LSP concern, so `line-index` belongs here even if `ropey` is deferred.

## Server capability surface for the bounded MVP

The current design note for the first user-visible handler slice is [`docs/lsp/02-design/bounded-mvp-handlers.md`](bounded-mvp-handlers.md).

The first server should advertise only the bounded MVP protocol surface:

- initialize / initialized / shutdown / exit
- text document open/change/close
- publish diagnostics
- document symbols
- completion
- go to definition
- find references

Recommended initial capability shape:

- `text_document_sync`: full sync
- `document_symbol_provider`: true
- `completion_provider`: basic support without resolve-first complexity
- `definition_provider`: true
- `references_provider`: true

Leave these off initially:

- `rename_provider`
- `hover_provider`
- `workspace_symbol_provider`
- `semantic_tokens_provider`
- `code_action_provider`

The point is to keep server claims aligned with the bounded semantic surface already documented elsewhere.

## State model

The server state should be organized around four buckets.

### 1. Session state

- `tower_lsp::Client`
- negotiated client capabilities
- workspace roots
- optional future config

### 2. Open document state

- current text
- current document version
- line index
- whether the document is file-backed or transient

### 3. Latest analysis facts

- latest `DocumentSnapshot` per document
- latest workspace/index facts derived from the analysis layer
- a record of which document versions those facts correspond to

### 4. Publishable outputs

- latest published diagnostics per document, if later deduplication becomes useful
- optional later caches for symbol/completion responses if needed

The important point is that handlers should read from cached analysis outputs rather than recomputing everything ad hoc.

## Workspace-root handling

The first server should normalize workspace roots from:

1. `initialize.workspaceFolders` when present
1. `initialize.rootUri` as fallback

Those roots should be stored explicitly in state rather than being rediscovered in each handler.

The bounded MVP can treat multiple roots conservatively:

- scan/index each root independently
- avoid cross-root semantic assumptions
- do not pretend every open file belongs to one shared global workspace

That matches the workspace-instance model already documented for includes.

## Handler flow

The first useful handler flow should look like this.

### `initialize`

Should:

- capture client capabilities
- capture workspace roots
- advertise bounded-MVP capabilities

Should not:

- eagerly do heavyweight analysis before the client can finish startup

### `initialized`

Should:

- kick off or schedule the first workspace scan/index if workspace roots exist

This keeps initialization fast while still letting the server begin warming caches.

### `didOpen`

Should:

- store the full current document text and version
- analyze the document via the analysis crate
- store the resulting snapshot
- publish syntax diagnostics immediately
- trigger any needed workspace/index refresh when include-aware or cross-file facts matter

### `didChange`

Should:

- replace the stored document text/version
- reanalyze the whole document
- replace the stored snapshot only if the result still matches the latest version
- republish diagnostics

That version check matters so slower analysis results do not overwrite newer document state.

### `didClose`

Should:

- drop transient open-buffer state
- keep or rebuild file-backed workspace facts through the workspace/index layer later
- avoid assuming close means “remove all knowledge of this file”

This matters because workspace features may still need on-disk files after editors close them.

### request handlers

`documentSymbol`, `completion`, `definition`, and `references` should:

- read the latest matching snapshot/index facts
- convert those facts into LSP responses
- return empty or partial results rather than guessing when the request falls into explicitly deferred scope

This is where the bounded-MVP discipline needs to stay strongest.

## Diagnostics policy in the LSP layer

The diagnostics policy from the overview should become concrete here.

### Publish syntax diagnostics first

On open/change, publish parse-derived diagnostics immediately from the latest document snapshot.

### Add semantic diagnostics only when supported facts exist

Only layer on:

- unresolved-reference diagnostics
- duplicate-definition diagnostics
- missing/cyclic include diagnostics

when the analysis/index layer can produce them reliably for the bounded scope.

### Keep include diagnostics separate

Missing/cyclic include diagnostics should be attached to the directive site in the including document.

They should not be conflated with:

- parse errors in included files
- unresolved identifiers
- broader runtime validation

## Concurrency and stale-result rules

The first implementation should stay conservative.

Recommended rules:

- use one shared async state container (`Arc<tokio::sync::RwLock<_>>` or similarly simple structure)
- do not hold locks across analysis work or other long async operations
- tag analysis results with the document version they were computed from
- discard results that arrive for stale versions

One more important point:

- the LSP crate should cache **analysis outputs**
- it should not cache or share raw parser internals in a way that forces cross-task parser reuse

That keeps the server easier to reason about while the architecture is still settling.

## Testing strategy

The testing strategy should mirror the crate split.

### Most tests should sit below JSON-RPC transport complexity

Preferred early test shape:

- instantiate the backend/state via the library API
- drive lifecycle and handler entrypoints directly where practical
- assert on typed LSP results and published-diagnostic payloads

This keeps most failures readable and focused.

### Keep a smaller set of protocol-shaped tests

For transport-facing confidence, add:

- `tokio::test`
- `serde_json`
- `assert-json-diff`

for a few protocol request/response smoke tests.

These should verify:

- capability advertisement
- lifecycle flow
- diagnostics publication shape
- one or two navigation/completion request shapes

### Reuse workspace and DSL fixtures

Do not invent a second DSL fixture tree inside the LSP crate.

Prefer:

- repo-root DSL fixtures for source inputs
- analysis-layer workspace fixtures when they exist
- LSP tests that consume those same fixtures through the analysis crate

## Zed-facing launch expectations

The first binary should assume:

- stdio transport
- a local binary path during development
- downstream packaging by the Zed extension later

That means the crate should avoid:

- editor-specific runtime assumptions
- hard-coding extension paths
- packaging logic in the server binary itself

The extension should remain responsible for launching and packaging.

The LSP crate should remain responsible for being a good stdio server.

## What this crate should not do

The first LSP crate should not:

- execute Structurizr runtime features such as `!script` or `!plugin`
- replace Tree-sitter-native highlighting/folding/indentation behavior
- reimplement analysis logic already owned by the analysis crate
- overpromise unsupported semantic scope such as `this`, hierarchical selectors, or dynamic relationship references
- require the Zed extension to change how it builds or consumes the grammar

## Recommended implementation sequence

1. Add the workspace member for [`crates/structurizr-lsp/`](../../../crates/structurizr-lsp/).
1. Create the library + binary skeleton with `tower-lsp-server` stdio bootstrap.
1. Add state, capability, and text-sync scaffolding.
1. Wire document open/change/close to whole-document analysis snapshots.
1. Publish syntax diagnostics first.
1. Add document symbols and keyword/directive completion.
1. Add bounded go-to-definition and find-references on top of analysis facts.
1. Only then layer on richer workspace-driven semantic diagnostics and later features.

This keeps the first server useful while preserving the clean analysis/LSP split.

## What this unblocks

Once this crate skeleton exists, future implementation no longer needs to guess:

- where protocol glue should live
- where document versions and workspace roots should live
- how analysis facts cross the boundary into LSP results
- how the future Zed extension should think about launching the server

It gives the project a clean top-layer crate that is editor-facing without becoming editor-specific.

## Sources

- [`docs/lsp/03-delivery/roadmap.md`](../03-delivery/roadmap.md)
- [`docs/lsp/01-foundations/overview.md`](../01-foundations/overview.md)
- [`docs/lsp/01-foundations/capability-matrix.md`](../01-foundations/capability-matrix.md)
- [`docs/lsp/01-foundations/repository-topology.md`](../01-foundations/repository-topology.md)
- [`docs/lsp/02-design/analysis-crate-skeleton.md`](analysis-crate-skeleton.md)
- [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md)
- `/Users/rob/dev/zed-structurizr/extensions.toml`
