# Structurizr DSL LSP roadmap

This roadmap assumes the language server remains an editor-oriented layer built on top of the current Tree-sitter parser crate, not a replacement for it.

It is intentionally more concrete than the surrounding overview docs so future sessions can pick up implementation work without having to rediscover the basic delivery sequence.

## Current repository status

The workspace now already contains:

- `crates/structurizr-analysis/` for transport-agnostic document and workspace analysis
- `crates/structurizr-lsp/` for the bounded MVP language server
- `crates/structurizr-cli/` for the unified `strz` binary, including `strz server`

That means the roadmap's early phases are no longer speculative crate-creation work.
They are useful as boundary and sequencing references, but the current delivery focus
starts after the in-repo bounded MVP:

- workspace-aware bounded semantics are implemented in-repo
- cross-file definition and references for the bounded identifier set are implemented
- bounded semantic diagnostics now sit alongside syntax and include diagnostics
- the next delivery step starts at downstream editor wiring and release choreography

## Recommended baseline decisions

- Keep grammar + analysis crate + LSP crate in this repository.
- Keep the current Zed extension as a separate downstream repo unless there is proven maintenance pain.
- Reuse the existing Tree-sitter grammar and Rust bindings directly.
- Use `tower-lsp-server` first, with `tower_lsp_server::ls_types` as the protocol type surface inside the LSP crate.
- Keep the analysis crate free of LSP types.
- Let Tree-sitter queries continue to power editor-native syntax features where Zed already handles them well.
- Focus the LSP on diagnostics, symbols, navigation, references, and completion before semantic tokens or runtime-style validation.

## Current layout

The repository now already has this broad shape:

```text
bindings/rust/                  existing parser crate surface
crates/structurizr-analysis/    semantic extraction and indexing
crates/structurizr-lsp/         language server
docs/lsp/                       architecture and roadmap docs
tests/                          parser + analysis + protocol fixtures
```

With the current Zed extension remaining a separate downstream repository that pins this grammar repo and launches the released or locally built `strz server`.

## Phase 0: lock the boundaries and development loop

This phase is about removing ambiguity before code starts.

### 0.1 Confirm responsibility boundaries

Write down and keep stable:

- what belongs in the grammar
- what belongs in the analysis crate
- what belongs in the LSP crate
- what remains Zed-extension-owned

Concrete outputs:

- `docs/lsp/01-foundations/overview.md`
- `docs/lsp/01-foundations/repository-topology.md`
- a short diagnostics policy note inside the LSP docs

### 0.2 Confirm repo topology

Make the near-term topology an explicit decision:

- grammar + analysis + LSP in this repo
- Zed extension in `/Users/rob/dev/zed-structurizr` / `https://github.com/sinon/zed-structurizr`

Concrete outputs:

- documented recommendation to keep the grammar independently buildable from repo root
- documented rule that the LSP must not force a non-standard grammar layout that breaks Zed consumption

### 0.3 Define the local development loop

Document how contributors iterate quickly across the two repos:

- edit grammar and LSP in this repo
- point the dev extension at `file:///Users/rob/dev/tree-sitter-structurizr` for grammar changes
- point the dev extension at a local LSP binary for server changes
- pin commit SHAs and package binaries only when preparing an extension release

Concrete outputs:

- a written local-dev flow in the docs
- later, a contributor command surface or script if the manual loop becomes painful

### 0.4 Define MVP feature boundaries

State explicitly that MVP supports:

- syntax diagnostics
- document symbols
- keyword/directive completion
- go-to-definition for bounded identifier cases
- find-references for the same bounded identifier cases

State explicitly that MVP excludes:

- `this`
- hierarchical selectors such as `a.b.c`
- named dynamic-view relationship references
- rename for unresolved scope shapes
- semantic tokens
- runtime-style validation

### Exit criteria

- contributors can answer “where does this logic belong?” consistently
- the repo-topology decision is documented
- the local grammar/LSP/Zed dev loop is documented
- the MVP boundary is explicit enough that the first implementation cannot silently expand

## Phase 1: harden the grammar and query surface for analysis

This phase makes sure the parse tree is stable enough to analyze.

For a tighter execution list, see `docs/lsp/90-history/phase1-backlog.md`.

### 1.1 Audit symbol-bearing syntax

Review the current parse-tree shapes for the constructs the first analyzer will care about:

- identifier assignments such as `a = softwareSystem "A"`
- direct relationships
- named relationships
- `!include`
- `!identifiers`
- direct view references that may later matter for hover/definition

Concrete outputs:

- a short checklist of node kinds and fields that the analyzer will rely on
- a list of any grammar gaps that would block symbol extraction

### 1.2 Add or tighten fixtures for future analysis work

Add realistic fixture coverage for:

- a minimal multi-file workspace using `!include`
- direct identifier definitions and uses
- named relationships
- `!identifiers`
- deliberately unsupported cases that should stay deferred

Suggested future fixture areas:

- `tests/fixtures/lsp/`
- `tests/fixtures/workspace/`
- `test/corpus/` additions for focused syntax slices

Concrete outputs:

- at least one real multi-file fixture, not just parse-only include syntax
- snapshots that make future symbol extraction easier to reason about

### 1.3 Decide query ownership and add the first missing query surfaces

Decide deliberately which queries belong:

- in this grammar repo as portable query surfaces
- in the Zed extension as editor-specific behavior

Likely early additions:

- `queries/tags.scm`
- optionally `queries/outline.scm`
- optionally `queries/brackets.scm`

If a query is Zed-specific rather than generally portable, keep it in the extension repo instead of forcing it upstream.

### 1.4 Validate grammar support against the intended MVP feature set

Before moving on, confirm the grammar cleanly represents:

- top-level assigned identifiers
- direct model element references
- relationship declarations that may later become definable/referenceable
- include directives in a way the analyzer can extract file paths from

### Exit criteria

- parse trees for symbol-bearing constructs are stable enough to analyze
- a minimal multi-file workspace fixture exists
- there is a clear answer for where `tags`, `outline`, and `brackets` queries should live
- the grammar covers the syntax needed for the bounded MVP navigation features

## Phase 2: create the analysis crate skeleton

This phase built the reusable semantic layer without any LSP transport code.

### 2.1 Introduce the workspace/crate structure

Add a new analysis crate while keeping the grammar independently consumable.

Plausible future shape:

```text
crates/structurizr-analysis/
  Cargo.toml
  src/lib.rs
  src/parse.rs
  src/snapshot.rs
  src/symbols.rs
  src/diagnostics.rs
  src/includes.rs
  src/queries.rs
```

The exact filenames can change, but the separation of concerns should stay clear.

The current design note for this phase is `docs/lsp/02-design/analysis-crate-skeleton.md`.

### 2.2 Define stable analysis data structures

The first pass should settle on types roughly like:

- `DocumentId`
- `DocumentSnapshot`
- `SyntaxDiagnostic`
- `Symbol`
- `Reference`
- `IncludeDirective`
- `WorkspaceFacts`

These types should stay transport-agnostic and avoid leaking `lsp-types`.

### 2.3 Implement parse + syntax-diagnostic extraction

The analysis crate should be able to:

- parse source text using the existing grammar crate
- retain the resulting Tree-sitter tree
- extract syntax diagnostics from parse errors
- expose both raw-tree access and higher-level extracted facts

### 2.4 Implement first-pass symbol extraction

The initial extractor should only support the bounded MVP surface:

- top-level assigned identifiers
- direct model-element declarations
- direct identifier references in obvious cases
- include directives as extracted facts

Do **not** broaden into `this`, selectors, or named dynamic-view relationship references yet.

The current design note for this phase is `docs/lsp/02-design/first-pass-symbol-extraction.md`.

### 2.5 Add analysis-level tests

Use the current repo’s test style rather than inventing a new one.

Recommended test surfaces:

- `rstest` for fixture discovery
- `insta` snapshots for extracted symbols/diagnostics
- dedicated realistic DSL fixtures under `tests/fixtures/`

### Exit criteria

- single-file analysis works without any LSP code
- syntax diagnostics and symbol facts can be snapshot-tested directly
- the first-pass extracted facts are stable enough to power later LSP handlers

## Phase 3: write down scope rules and build workspace indexing

This phase is where the “bounded MVP” promise needs to stay disciplined.

### 3.1 Write the first scope-rules note

Document exactly what is supported first:

- top-level assigned identifiers
- direct model element references
- include-file presence and cycle handling

Document exactly what is deferred:

- `this`
- hierarchical selectors
- named dynamic-view relationship references
- deeper view/model scoping questions

Concrete outputs:

- a scope-rules note in `docs/lsp/`
- fixtures that correspond to each supported and deferred case

The current design note for this phase is `docs/lsp/02-design/scope-rules.md`.

### 3.2 Build workspace discovery

Use `ignore` for workspace scanning of `.dsl` files.

Important rule:

- explicit `!include` targets should be resolved even if general workspace walking respects ignore rules

Concrete outputs:

- a workspace loader that can find candidate `.dsl` files
- a resolver that can follow explicit include targets
- `docs/lsp/02-design/workspace-discovery-includes.md`

### 3.3 Add include resolution and diagnostics

Start with file-level behavior only:

- resolve include paths
- report missing files
- detect cycles

Do not treat this as full runtime-style include semantics yet.

The current design note for this phase is `docs/lsp/02-design/workspace-discovery-includes.md`.

### 3.4 Merge per-file facts into a workspace index

Build a workspace view that can answer:

- what symbols are defined in which file
- what references point at which definitions for the bounded identifier set
- which includes succeed or fail

The current design note for this phase is `docs/lsp/02-design/workspace-index.md`.

### 3.5 Add workspace-level tests

Suggested future test shape:

```text
tests/lsp/workspaces/
  minimal/
  includes/
  duplicates/
  unresolved/
```

The names are illustrative; the important thing is to have realistic multi-file workspaces as fixtures.

### Exit criteria

- definitions and references work across a realistic multi-file workspace for the bounded identifier set
- missing-file and cycle diagnostics exist for includes
- workspace facts are stable enough that LSP handlers can read from them instead of walking raw trees ad hoc

## Phase 4: build the LSP crate and ship the bounded MVP

This phase is protocol wiring, state management, and careful feature scoping.

### 4.1 Introduce the LSP crate

Plausible future shape:

```text
crates/structurizr-lsp/
  Cargo.toml
  src/main.rs
  src/server.rs
  src/state.rs
  src/documents.rs
  src/handlers/
    diagnostics.rs
    symbols.rs
    completion.rs
    goto_definition.rs
    references.rs
```

Again, the exact file names can change, but the server should not collapse into one undifferentiated module.

The current design note for this phase is `docs/lsp/02-design/lsp-crate-skeleton.md`.

### 4.2 Define the server state model

The LSP state should be organized around:

- open documents
- document text buffers
- latest parsed snapshots
- workspace facts/indexes
- client capabilities that matter for feature behavior

This is where `ropey`, `line-index`, and `tower-lsp-server` start earning their keep.

### 4.3 Implement the protocol skeleton

Implement first:

- `initialize`
- `initialized`
- `shutdown`
- open/change/close document flow
- workspace reload/index refresh hooks as needed

### 4.4 Implement diagnostics first

Surface syntax diagnostics before semantic diagnostics.

Then add bounded semantic diagnostics:

- unresolved references for supported identifier cases
- duplicate definitions for supported identifier cases
- missing/cyclic include diagnostics

### 4.5 Implement the first user-visible navigation features

Implement in this order:

1. document symbols
2. keyword/directive completion
3. go-to-definition for top-level assigned identifiers and direct model element references
4. find-references for the same bounded identifier set

If a feature request reaches into deferred scope cases, return to the scope-rules note rather than guessing.

The current design note for this handler slice is `docs/lsp/02-design/bounded-mvp-handlers.md`.

### 4.6 Add LSP-level tests

Recommended test approach:

- async handler tests with `tokio::test`
- protocol payload assertions with `serde_json`
- payload comparisons with `assert-json-diff`
- realistic workspace fixtures shared with the analysis layer where possible

### 4.7 Run the transport checkpoint

After the MVP server works in practice, decide whether `tower-lsp-server` still feels like the right transport layer.

Do **not** optimize away from it prematurely, but do review:

- cancellation behavior
- handler ergonomics
- test ergonomics
- any protocol-control limitations that matter for future work

### Exit criteria

- the bounded MVP features work against realistic `.dsl` workspaces
- the server is useful without pretending to be a Structurizr runtime
- there is enough confidence to wire the server into the Zed extension

## Phase 5: wire the bounded MVP into the existing Zed extension

This phase treats the extension as a downstream consumer and packaging layer.

### 5.1 Extend the existing extension manifest

In `/Users/rob/dev/zed-structurizr`, add the language-server registration and keep the grammar registration aligned with the grammar repo.

Concrete outputs:

- updated `extension.toml`
- grammar pin strategy that matches the intended release process

### 5.2 Add extension-side Rust code if needed

The extension will need to launch the server in the way Zed expects.

Concrete output:

- extension-side code that implements `language_server_command`

### 5.3 Define the local integration workflow

Document the expected local-dev flow:

- `file://` grammar override for local grammar changes
- local LSP binary path for server changes
- smoke-test document or workspace to exercise basic navigation and diagnostics

### 5.4 Decide packaging strategy

Choose between:

- downloaded release binaries per platform
- or a user-installed LSP binary with extension wiring

This is a product/distribution decision as much as a technical one, so it should be made explicitly instead of drifting.

The current design note for this local-dev and packaging slice is `docs/lsp/03-delivery/packaging-and-dev-loop.md`.

### 5.5 Add extension smoke tests and manual checks

At minimum, validate:

- grammar still loads
- editor-side highlighting/folding/outline behavior still works
- document symbols work
- go-to-definition works on bounded identifier cases
- include diagnostics appear sensibly

The current design note for this phase is `docs/lsp/03-delivery/zed-extension-language-server-wiring.md`.

### Exit criteria

- a contributor can run the extension locally against this repo’s grammar and LSP
- the extension still benefits from Tree-sitter-native editor features while the LSP adds semantic value

## Phase 6: grow the server carefully after MVP

Only expand once the bounded MVP is stable.

The current design note for this phase is `docs/lsp/03-delivery/advanced-semantic-expansion.md`.

### 6.1 Add narrow syntax-backed completion refinements

A good first post-MVP refinement was style-property completion inside parsed `element_style` and `relationship_style` blocks.

That property-name slice is now landed.
The remaining follow-up work here should stay narrow:

- finite value completion where the grammar already fixes the allowed value set
- optional `properties {}` scaffolding inside style-rule blocks
- any further syntax-backed tables that still stay separate from semantic identifier completion and `!identifiers` policy

### 6.2 Add richer semantic features

Candidates:

- rename
- richer hover
- workspace symbols
- broader reference coverage

Each one should be gated by whether the scope rules are strong enough to implement it safely.

### 6.3 Improve include and workspace diagnostics

Add better diagnostics quality, richer messages, and more workspace-awareness without drifting into full runtime validation.

### 6.4 Revisit query layering

Once the server exists, revisit whether any additional query surfaces should move:

- into the grammar repo as portable assets
- or stay in the Zed extension as editor-specific assets

### 6.5 Consider polish features

Possible later additions:

- semantic tokens
- code actions
- more advanced hover content

These should be treated as polish, not as proof that the server is viable.

### Exit criteria

- advanced features are driven by a tested semantic model rather than ad hoc request handlers
- the server remains clearly editor-oriented rather than turning into a runtime proxy

## Suggested testing layers across the whole roadmap

- keep existing grammar tests as the syntax baseline
- add analysis fixtures that snapshot extracted symbols and diagnostics
- add workspace fixtures for multi-file resolution and include handling
- add LSP integration tests that exercise request/response behavior over realistic workspaces
- continue using the upstream audit to drive grammar parity where semantic features depend on syntax coverage

## Risks to watch

- drifting into runtime behavior instead of editor behavior
- tying request handlers too closely to raw Tree-sitter node shapes
- under-specifying include and scope semantics
- under-specifying repo boundaries between grammar, LSP, and Zed extension
- duplicating features Zed already gets from Tree-sitter queries
- delaying delivery by chasing semantic tokens too early

## First in-repo implementation milestone

The first in-repo milestone was to prove this chain works end-to-end:

1. parse a `.dsl` file with the existing grammar
2. extract basic definitions/references for the bounded identifier set
3. surface syntax diagnostics
4. answer go-to-definition and find-references from the language server
5. validate the same bounded semantic model over realistic multi-file workspaces

That milestone is now landed in this repository.

## Next delivery milestone

The next piece of delivery work is:

1. wire the existing bounded MVP into the downstream Zed extension
2. point the extension at released or locally built `strz` binaries
3. smoke-test the release-binary flow against representative workspaces

If that milestone is solid, the rest of the editor integration can grow incrementally rather than as one large speculative build.
