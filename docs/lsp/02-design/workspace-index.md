# Structurizr DSL workspace index

> Status: implemented in bounded form.
>
> The workspace layer described here now exists in-repo. Read this note as the
> contract for the current bounded workspace view and the rationale for what is
> still intentionally deferred.

This note turns Phase 3.4 of [`docs/lsp/03-delivery/roadmap.md`](../03-delivery/roadmap.md) into a concrete design for how the future analysis crate should merge per-document facts into one bounded semantic workspace view.

It sits on top of:

- [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md)
- [`docs/lsp/02-design/analysis-crate-skeleton.md`](analysis-crate-skeleton.md)
- [`docs/lsp/02-design/first-pass-symbol-extraction.md`](first-pass-symbol-extraction.md)
- [`docs/lsp/02-design/scope-rules.md`](scope-rules.md)

The goal is to define a workspace-layer semantic model that is strong enough for bounded cross-file definition, references, and include diagnostics without drifting into a Structurizr runtime.

## Why this needs its own note

By the time Phase 3.4 starts, the future analysis crate should already have:

- immutable `DocumentSnapshot` values
- raw `!include` facts
- raw `!identifiers` facts
- structural and bindable `Symbol` facts
- observed `Reference` facts
- workspace instances rooted at entry documents

That is necessary, but it is still not enough for bounded semantic features.

The missing layer is the thing that answers:

- which bindable symbol wins for a given canonical key
- whether a key is duplicated and therefore unsafe to navigate to
- whether a reference resolves, does not resolve, or is ambiguous
- which workspace instances a shared fragment belongs to

That is the workspace index.

## Main conclusion

The bounded MVP should build a **derived workspace index per workspace instance**, not one folder-global symbol table and not one ad hoc query per handler.

That index should:

1. collect bindable element and relationship symbols from the instance's document set
1. compute final canonical binding keys in **instance context**
1. classify keys into unique bindings vs duplicate bindings
1. resolve supported reference facts against those binding tables
1. retain per-document include and semantic diagnostic facts
1. expose reverse document-to-instance membership so fragment queries can stay conservative

This makes the workspace layer the semantic bridge between single-document extraction and LSP handlers.

## Bounded goal

The workspace index only needs to support the bounded MVP.

It should be able to answer:

- what symbols are defined in which file for the bounded identifier set
- which supported references resolve to which definitions
- which supported references do not resolve or are ambiguous
- which includes succeed, fail, or participate in cycles
- which workspace instances a given document belongs to

It should **not** try to settle:

- selector resolution such as `system.api`
- `this` semantics
- dynamic relationship-reference behavior
- full runtime validation
- every broader naming or execution rule in upstream Structurizr

## Why per-document facts are not enough

Single-document extraction is intentionally narrower than semantic resolution.

That split is correct, because:

- reference extraction does not know about other files
- duplicate detection is meaningless without an instance-wide symbol table
- a fragment may belong to multiple workspace instances
- effective `!identifiers` mode may depend on workspace context outside the current file

This last point is especially important.

## Canonical keys are not always purely document-local

The scope rules note defines canonical flat vs hierarchical element keys and flat relationship keys.

But the final canonical key for a bindable symbol cannot always be frozen inside `DocumentSnapshot`.

Why:

- model-level `!identifiers` overrides are local to the document context where they appear
- workspace-level `!identifiers` may be inherited from an entry document
- one physical fragment can be included by multiple roots with different workspace-level identifier modes

That means the same `binding_name` in the same file can produce different final element keys across workspace instances.

So the right split is:

- `DocumentSnapshot` stores raw facts needed to build keys later
- `WorkspaceIndex` computes final canonical binding keys in instance context

## Important bounded-MVP nuance about include placement

The current grammar only allows `include_directive` at:

- `source_file`
- `workspace_block`
- `model_block`

So for the bounded MVP:

- include expansion does **not** currently splice a fragment into a parent declaration body
- `parent_symbol` chains for core declarations remain document-local

That means instance-time key construction must account for inherited identifier mode, but it does **not** yet need to stitch together a declaration ancestry chain that crosses file boundaries.

If later grammar-alignment work broadens include placement, this assumption should be revisited explicitly.

## Recommended ownership split

The future analysis crate should own:

- workspace-instance construction
- final binding-table construction
- duplicate classification
- supported reference resolution
- reverse document-to-instance membership
- workspace-level semantic diagnostics as stable facts

The future LSP crate should own:

- choosing the relevant workspace instance(s) for a request
- converting workspace-index facts into LSP responses
- deciding when to rebuild or invalidate cached indexes after file events

This keeps semantic policy transport-agnostic.

## Core model

The cleanest shape is to distinguish:

- one `WorkspaceInstance`
- from one derived `WorkspaceIndex`
- from one top-level `WorkspaceIndexSet`

## `WorkspaceInstance`

This is the structural expansion rooted at one entry document:

- root document ID
- transitive local include closure
- resolved include edges
- stable document order for deterministic processing

This is already implied by [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md).

## `WorkspaceIndex`

This is the semantic view derived from one `WorkspaceInstance`.

It should conceptually contain:

- instance identity
- documents participating in the instance
- bindable symbol handles grouped into element vs relationship tables
- duplicate key sets
- resolved reference facts
- unresolved/ambiguous reference facts
- include diagnostics and later semantic diagnostics grouped by document

## `WorkspaceIndexSet`

This is the host analysis object for all known indexes.

It should conceptually contain:

- all known workspace indexes keyed by instance ID
- reverse mapping from `DocumentId` to candidate `WorkspaceInstanceId`s
- enough metadata for invalidation/rebuild decisions

This is the object the future LSP state can cache and query.

## Recommended fact shapes

The workspace layer should keep reusing document-local facts rather than duplicating whole symbol payloads.

Useful conceptual handle types:

| Type                        | Role                                                          |
| --------------------------- | ------------------------------------------------------------- |
| `WorkspaceInstanceId`       | Stable identity for one root-driven semantic instance         |
| `SymbolHandle`              | Stable reference to one extracted `Symbol` in one document    |
| `ReferenceHandle`           | Stable reference to one extracted `Reference` in one document |
| `CanonicalBindingKey`       | Final instance-scoped key used for exact-match lookup         |
| `ResolvedReference`         | One supported reference site plus its resolved target symbol  |
| `ReferenceResolutionStatus` | Outcome of attempting bounded resolution                      |
| `DuplicateBindingSet`       | One canonical key plus all symbol handles claiming it         |

Important rule:

- workspace facts should point back to `DocumentSnapshot` facts by handle instead of copying large symbol/reference payloads everywhere

That keeps the workspace layer derived and cache-friendly.

## Stable processing order

The workspace index should process documents in a deterministic order.

Recommended rule:

1. root document first
1. then included documents in resolved traversal order
1. with directory includes already expanded deterministically as required by the include note

Why this matters:

- deterministic tests
- deterministic duplicate-diagnostic ordering
- deterministic reverse indexes

Why this should **not** mean:

- “earlier definitions win”

Stable order is for reproducibility, not for semantic tie-breaking.

## Building the index in phases

The future implementation should build the workspace index in four small phases.

## Phase A: collect document-local inputs

For every document in the workspace instance:

- read `DocumentSnapshot`
- read extracted `Symbol` facts
- read extracted `Reference` facts
- read raw `IdentifierModeFact` values
- read include diagnostics and include edges

At this stage, the workspace layer still has raw ingredients, not final resolution.

## Phase B: compute canonical binding keys in instance context

This is where the scope rules become real.

For each bindable symbol in the instance:

- determine the effective identifier mode for the symbol's declaration context
- compute its canonical element or relationship key
- record the mapping from key to `SymbolHandle`

### Element symbols

Element binding keys should be computed per the scope rules note:

- `flat` -> local assignment identifier
- `hierarchical` -> ancestor element path joined with `.`
- groups remain transparent

### Relationship symbols

Relationship binding keys stay flat:

- canonical key = local relationship identifier

### Why this belongs here

This is the point where the workspace layer knows:

- which entry document is active
- which workspace-level identifier directive applies
- whether one shared fragment is being viewed in one or many semantic contexts

That is why final key construction belongs in the workspace index instead of being hard-coded into single-document extraction.

## Phase C: classify unique vs duplicate bindings

The index should keep element and relationship binding tables separate, exactly as the scope rules note requires.

Within each table:

- one symbol for a key -> unique binding
- multiple symbols for a key -> duplicate binding set

Recommended conceptual tables:

- `unique_element_bindings`
- `duplicate_element_bindings`
- `unique_relationship_bindings`
- `duplicate_relationship_bindings`

### No winner policy

For the bounded MVP, duplicate bindings should have **no semantic winner**.

Do **not** use:

- include order
- traversal order
- “same document first”
- display-name heuristics

to silently choose one definition.

Instead:

- keep the duplicate set explicit
- let later semantic diagnostics surface the conflict
- return no confident navigation answer for references that depend on that key

This matches the project’s editor-tooling goal better than accidentally imitating runtime behavior we have not chosen to model.

## Phase D: resolve supported references

Only the supported bounded reference kinds should be resolved:

- `RelationshipSource`
- `RelationshipDestination`
- `ViewScope`
- `ViewInclude`
- `ViewAnimation`

And they should be resolved exactly as the scope note describes.

### Relationship endpoints

Resolve against:

- unique element bindings only

Outcomes:

- one unique match -> resolved
- zero matches -> unresolved
- duplicate key set -> ambiguous

### View scope

Resolve against:

- unique element bindings only

Outcomes:

- one unique match -> resolved
- zero matches -> unresolved
- duplicate key set -> ambiguous

### View include

Resolve against:

1. unique element bindings
1. unique relationship bindings

Outcomes:

- exactly one candidate total -> resolved
- zero candidates -> unresolved
- candidate from both tables -> ambiguous
- duplicate key set in either relevant table -> ambiguous

This preserves the conservative `ElementOrRelationship` rule from the scope note.

## Reference resolution outcomes

The workspace layer should not reduce every failure to “not found”.

The first useful `ReferenceResolutionStatus` variants should be conceptually like:

- `Resolved(SymbolHandle)`
- `UnresolvedNoMatch`
- `AmbiguousDuplicateBinding`
- `AmbiguousElementVsRelationship`
- `DeferredByScopePolicy`

The exact enum names can change, but the distinction matters because:

- diagnostics may later want richer messages
- tests should be able to distinguish “missing” from “ambiguous”
- LSP handlers should keep returning conservative results without throwing away why

## Include diagnostics remain part of the workspace view

The include note already says include diagnostics should point at directive sites in the parent document.

The workspace index should carry those diagnostics forward as part of the per-document derived facts for the instance.

That means one `WorkspaceIndex` should be able to answer:

- what include diagnostics apply to document X in this instance
- what later semantic diagnostics apply to document X in this instance

Recommended future grouping:

- `diagnostics_by_document: BTreeMap<DocumentId, Vec<WorkspaceDiagnostic>>`

The exact container type can vary; the important part is that the result is grouped by document and deterministic.

## Open-fragment support depends on reverse membership

One of the most important outputs of the workspace layer is:

- `document -> candidate workspace instances`

This is what lets the LSP behave conservatively for shared fragments.

### Zero contexts

If a document belongs to zero known workspace instances:

- keep syntax-only behavior
- do not fabricate a workspace index for it

### One context

If a document belongs to exactly one instance:

- use that `WorkspaceIndex` directly

### Multiple contexts

If a document belongs to multiple instances:

- compare candidate semantic answers across those indexes
- only return a semantic result when the result is identical across all candidates

Examples:

- a definition request can succeed only if every candidate instance resolves to the same `SymbolHandle`
- a references request can succeed only if every candidate instance produces the same set of `ReferenceHandle`s

This keeps fragment behavior aligned with the already documented scope rules.

## Same-document fast paths must not bypass instance-wide uniqueness

The bounded handler note currently says definition logic should be “same-document first”.

That remains a useful implementation tactic before full workspace indexing exists.

But once a `WorkspaceIndex` exists, the final semantic answer should still respect instance-wide uniqueness.

That means:

- a same-document binding can be used as a cheap starting point
- but it should **not** be returned if the same canonical key is duplicated elsewhere in the active workspace instance

This is another reason the workspace layer should own duplicate classification rather than leaving it to handlers.

## Shared fragments and identifier mode

The first workspace-index implementation should carry one special scenario explicitly in mind:

- one fragment included by multiple roots
- different roots choose different workspace-level `!identifiers` mode

In that case:

- the fragment's raw `Symbol` facts stay the same
- the final canonical element keys may differ per instance
- the reverse document-to-instance mapping is what prevents the server from collapsing those contexts together

This scenario is subtle enough that it should have a dedicated future test.

## What the index should not own

The workspace layer should **not** take over responsibilities that belong elsewhere.

### Still document-local

- parse trees
- syntax diagnostics
- raw extracted `Symbol` and `Reference` facts
- raw directive facts

### Still LSP-layer

- URI/position conversion
- request routing
- choosing which candidate instances to compare for one editor action
- caching policy exposed to editor lifecycle events

### Still deferred

- selector reference semantics
- `this`
- dynamic relationships and their references
- rename edit planning
- identifier completion
- runtime-style validation

## Invalidation and recompute policy

The first implementation should favor correctness and determinism over micro-incrementality.

Recommended bounded policy:

- keep per-document snapshots cached
- keep include graph facts cached
- when one document changes, identify affected workspace instances
- rebuild each affected workspace index as a whole

This is good enough for the bounded MVP because:

- workspace instances are likely modest in size
- whole-instance rebuilds are easier to reason about than partial semantic patching
- it keeps invalidation logic aligned with include dependencies

Later profiling can decide whether narrower recompute is worthwhile.

## Example: current multi-file fixture slice

The current fixtures already suggest the first useful cross-file case:

[`crates/structurizr-lsp/tests/fixtures/includes/workspace_fragments-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/workspace_fragments-ok.dsl)

```dsl
workspace {
    !include "model-fragment-ok.dsl"
    !include "views-fragment-ok.dsl"
}
```

[`crates/structurizr-lsp/tests/fixtures/includes/model-fragment-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/model-fragment-ok.dsl)

```dsl
model {
    user = person "User"
    system = softwareSystem "System" {
        api = container "API"
    }

    user -> api "Uses"
}
```

[`crates/structurizr-lsp/tests/fixtures/includes/views-fragment-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/views-fragment-ok.dsl)

```dsl
views {
    container system "container-view" {
        include *
        autoLayout lr
    }
}
```

The workspace index for that root should be able to say:

- `user`, `system`, and `api` are element bindings in the same workspace instance
- the relationship endpoints in the model fragment resolve within that instance
- the `system` view scope in the views fragment resolves to the definition in the model fragment

That is the core “multi-file but still bounded” outcome this note is aiming for.

## Testing shape to add when implementation begins

The roadmap already suggests future workspace tests.

The most important scenarios for the first workspace index are:

- one root workspace including local file fragments
- cross-file view-scope resolution
- duplicate element identifier across two included files
- duplicate named relationship identifier across two included files
- one fragment included by multiple roots
- one shared fragment under different workspace-level `!identifiers` modes
- missing local include target
- include cycle

Suggested future area:

```text
tests/lsp/workspaces/
  minimal/
  includes/
  duplicates/
  shared-fragments/
  identifier-modes/
  missing/
  cycles/
```

The exact directory names can change, but these scenarios should be represented.

## Recommended implementation order

1. Keep `DocumentSnapshot` and extraction facts single-document and owned.
1. Build `WorkspaceInstance` values from discovery/include resolution.
1. Add `WorkspaceIndex` as a derived pass over one instance.
1. Add unique/duplicate binding tables.
1. Add supported reference resolution statuses.
1. Add reverse document-to-instance membership.
1. Only then layer cross-file definition/references and semantic diagnostics on top.

This keeps the analysis model staged and testable instead of merging everything into one opaque “workspace pass”.

## What this unblocks

Once this note is followed:

- the analysis crate has a concrete home for bounded cross-file semantics
- the scope rules become executable at workspace-instance granularity
- LSP handlers can query stable workspace facts instead of re-deriving symbol tables per request
- fragment reuse stays conservative instead of collapsing into one false global workspace

That is the semantic step that makes the planned LSP genuinely useful across multiple files.

## Sources

- [`docs/lsp/03-delivery/roadmap.md`](../03-delivery/roadmap.md)
- [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md)
- [`docs/lsp/02-design/analysis-crate-skeleton.md`](analysis-crate-skeleton.md)
- [`docs/lsp/02-design/first-pass-symbol-extraction.md`](first-pass-symbol-extraction.md)
- [`docs/lsp/02-design/scope-rules.md`](scope-rules.md)
- [`docs/lsp/02-design/bounded-mvp-handlers.md`](bounded-mvp-handlers.md)
- [`docs/lsp/90-history/syntax-audit-directive-nodes.md`](../90-history/syntax-audit-directive-nodes.md)
- [`crates/structurizr-lsp/tests/fixtures/includes/workspace_fragments-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/workspace_fragments-ok.dsl)
- [`crates/structurizr-lsp/tests/fixtures/includes/model-fragment-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/model-fragment-ok.dsl)
- [`crates/structurizr-lsp/tests/fixtures/includes/views-fragment-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/includes/views-fragment-ok.dsl)
