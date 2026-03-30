# Structurizr DSL LSP Phase 1 backlog

This document turns Phase 1 from `docs/lsp/03-delivery/roadmap.md` into a tighter execution backlog.

Its job is not to redesign the project again. Its job is to make the first implementation slice executable without another round of broad planning.

## Phase 1 goal

Make the grammar, fixtures, and query decisions stable enough that a future analysis crate can extract bounded symbols and diagnostics without guessing.

Phase 1 is complete when:

- the parse tree shape for bounded-MVP symbols is documented and trusted
- realistic fixtures exist for the first analysis targets
- query ownership between this repo and `zed-structurizr` is explicit
- the grammar is known to cover the syntax needed for bounded MVP navigation

## Bounded scope for Phase 1

Phase 1 should prepare for analysis of:

- top-level assigned identifiers such as `a = softwareSystem "A"`
- direct model-element declarations
- direct identifier references in obvious cases
- named relationships
- `!include`
- `!identifiers`

Phase 1 should **not** broaden into:

- `this`
- hierarchical selectors such as `a.b.c`
- dynamic-view relationship references
- rename semantics
- runtime-style validation

## Primary deliverables

Phase 1 should produce these concrete outputs:

1. A symbol-bearing syntax checklist for the bounded-MVP surface.
2. A realistic multi-file fixture set that analysis and LSP work can reuse later.
3. An explicit query ownership decision for `tags`, `outline`, and `brackets`.
4. A short list of grammar gaps, if any, that block bounded symbol extraction.

## Ordered work packages

## Work package 1: audit symbol-bearing node shapes

### Purpose

Confirm which node kinds and named fields the future analyzer can rely on.

### Files to inspect

- `crates/structurizr-grammar/grammar.js`
- `crates/structurizr-grammar/src/node-types.json`
- `crates/structurizr-grammar/tests/fixtures/**/*.dsl`
- `crates/structurizr-grammar/test/corpus/*.txt`

### Checklist to capture

The audit should record the node kind and important fields for:

- identifier assignment statements
- `person`, `softwareSystem`, `container`, `component`, and other direct declaration forms
- direct relationship forms
- named relationship forms
- `!include`
- `!identifiers`
- direct view references that may later need symbol resolution

### Expected output

A future doc or note containing:

- node kind
- named fields used by the analyzer
- representative fixture/corpus examples
- any ambiguities or unstable shapes

Current output from this audit slice:

- `docs/lsp/90-history/syntax-audit-assignment-declarations.md`
- `docs/lsp/90-history/syntax-audit-reference-relationship-nodes.md`
- `docs/lsp/90-history/syntax-audit-directive-nodes.md`

### Exit condition

The future analysis crate can point to a stable syntax contract instead of re-deriving it from raw trees.

## Work package 2: add analysis-oriented fixtures

### Purpose

Create realistic inputs that the analysis crate and LSP can reuse later.

### Fixture groups to add or tighten

#### 2.1 Minimal include-based workspace

Create a small multi-file workspace that does more than parse a literal `!include` token.

Suggested shape:

```text
crates/structurizr-grammar/tests/fixtures/lsp/includes/workspace_fragments-ok.dsl
crates/structurizr-grammar/tests/fixtures/lsp/includes/model-fragment-ok.dsl
crates/structurizr-grammar/tests/fixtures/lsp/includes/views-fragment-ok.dsl
```

The exact names can change, but the fixture should prove:

- include paths appear in realistic workspace structure
- included files can carry model/view content
- the repo has a reusable multi-file example for later workspace indexing tests

#### 2.2 Identifier definition/reference fixtures

Add focused fixtures for:

- top-level assigned identifiers
- direct references to those identifiers
- direct declaration/reference combinations across model sections

Suggested shape:

```text
crates/structurizr-grammar/tests/fixtures/lsp/identifiers/assigned-identifiers-ok.dsl
crates/structurizr-grammar/tests/fixtures/lsp/identifiers/direct-references-ok.dsl
```

#### 2.3 Named relationship fixtures

Add fixtures that isolate:

- relationship assignment
- later reference to the named relationship where the grammar already supports it
- cases that should remain explicitly deferred

Suggested shape:

```text
crates/structurizr-grammar/tests/fixtures/lsp/relationships/named-relationships-ok.dsl
crates/structurizr-grammar/tests/fixtures/lsp/relationships/named-relationship-dynamic-reference-ok.dsl
```

The second fixture should remain parse-covered but semantically deferred for the bounded MVP.

#### 2.4 `!identifiers` fixtures

Add fixtures that make the directive visible and easy to reason about later when completion/rename behavior is designed.

Suggested shape:

```text
crates/structurizr-grammar/tests/fixtures/lsp/directives/identifiers-directive-ok.dsl
```

### Current output from this fixture slice

- `crates/structurizr-grammar/tests/fixtures/lsp/includes/workspace_fragments-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/includes/model-fragment-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/includes/views-fragment-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/identifiers/assigned-identifiers-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/identifiers/direct-references-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/relationships/named-relationships-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/relationships/named-relationship-dynamic-reference-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/lsp/directives/identifiers-directive-ok.dsl`

### Exit condition

Future analysis and LSP work can build on realistic fixtures instead of synthetic examples invented at implementation time.

## Work package 3: decide query ownership

### Purpose

Avoid drifting into duplicate or conflicting query definitions between this repo and the Zed extension.

### Queries to decide explicitly

- `tags.scm`
- `outline.scm`
- `brackets.scm`

### Decision criteria

Put a query in this repo if it is:

- portable across editors
- useful to non-Zed consumers
- naturally part of the grammar's reusable surface

Keep a query in `zed-structurizr` if it is:

- Zed-specific behavior or tuning
- tightly coupled to extension UX rather than the grammar's reusable shape

### Expected output

A short decision table answering:

- where each query should live
- why it lives there
- whether the other repo should mirror or consume it

Current output from this work package:

- `docs/lsp/01-foundations/query-ownership.md`

### Exit condition

Future query additions do not start with “which repo should this go in?” every time.

## Work package 4: review grammar gaps against bounded MVP needs

### Purpose

Check whether the current grammar cleanly supports the exact bounded-MVP navigation surface.

### Questions to answer

- Are top-level assigned identifiers represented cleanly enough for extraction?
- Are direct model-element references represented consistently?
- Are named relationships represented with enough structure to defer or support them deliberately?
- Are `!include` paths extractable without brittle text slicing?
- Is `!identifiers` represented clearly enough for later semantic decisions?

### Output

Either:

- “no blocking grammar gaps for bounded MVP analysis”

or:

- a short backlog of grammar changes required before the analysis crate should begin

### Exit condition

Phase 2 starts from a known syntax surface instead of discovering missing parse-tree structure mid-implementation.

## Work package 5: run the validation loop

### Purpose

Keep Phase 1 changes grounded in the repository's normal validation surface.

### Commands

For grammar/query/fixture edits:

```sh
just test-rust-fast
just test-grammar
```

If `crates/structurizr-grammar/grammar.js` changes:

```sh
just generate
just test-grammar
just test-rust-fast
```

If fixture snapshots intentionally change:

```sh
INSTA_UPDATE=always just test-rust
```

### Exit condition

Phase 1 artifacts are validated using the repo's normal contributor flow, not a one-off process.

## Recommended execution order

Use this order unless a concrete discovery forces a change:

1. audit symbol-bearing node shapes
2. add or tighten realistic fixtures
3. decide query ownership
4. review grammar gaps against bounded MVP needs
5. run the validation loop

## Suggested first implementation slice

If someone wants the smallest useful Phase 1 slice, do this first:

1. document the node shapes for top-level assigned identifiers, direct declarations, `!include`, and `!identifiers`
2. add one real multi-file include fixture
3. decide the home of `tags.scm`

That slice de-risks the future analysis crate much more than starting with broad query or LSP work.

## Phase 1 definition of done

Phase 1 is done when all of the following are true:

- the bounded-MVP syntax contract is documented
- multi-file include-based fixtures exist
- identifier and named-relationship fixtures exist
- query ownership for `tags`, `outline`, and `brackets` is explicit
- any grammar blockers for bounded MVP analysis are either fixed or written down as blocking tasks
- the normal validation commands pass
