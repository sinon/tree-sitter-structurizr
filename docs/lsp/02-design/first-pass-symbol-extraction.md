# Structurizr DSL first-pass symbol extraction

> Status: implemented in bounded form.
>
> The first bounded extraction pass now exists in-repo. Read this note as the
> contract that explains the current extraction choices and deferred cases.

This note defines the first bounded symbol and reference extraction pass for the future analysis crate.

It sits between:

- the syntax audits, which describe the parse-tree contract
- the analysis crate skeleton, which describes the crate/module boundaries
- the future LSP handlers, which will consume extracted facts rather than walking raw trees

The goal is to make the first implementation pass precise enough that we can build `DocumentSnapshot` facts without inventing extraction rules ad hoc.

## Why this needs its own note

The phrase “first-pass symbol extraction” sounds smaller than it is.

To make later document symbols, go-to-definition, and find-references work predictably, the analysis crate needs a clear answer for:

- which declarations become symbols
- which identifier sites become references
- which containers we traverse
- which syntax shapes we explicitly skip for the bounded MVP

Without that contract, the analysis crate and the future LSP handlers will drift into each making their own assumptions.

## Main conclusion

The first pass should extract **owned declaration symbols** and **owned observed reference sites** from a single document, without trying to resolve those references yet.

That means:

- declaration extraction is broader than “assigned identifiers only”
- binding extraction is narrower than “every declaration is definable”
- reference extraction is broader than relationship endpoints alone
- resolution remains a later workspace-instance concern

In practice, the first pass should:

- emit declaration symbols for core element declarations
- record the optional binding identifier for those declarations when present
- emit a symbol for named relationships when `relationship.identifier` exists
- emit reference facts for simple identifier sites in model relationships and obvious view fields
- explicitly defer `this`, selectors, nested shorthand, and dynamic relationship references

## Bounded goal

The first extraction pass should be sufficient to support later work on:

- document symbols for the core element tree
- go-to-definition for straightforward assigned identifiers
- find-references for the same bounded identifier set
- named relationship navigation where the reference site is syntax-simple and intentionally in scope

It should **not** attempt to solve:

- final reference resolution across files
- rename planning
- selector resolution such as `a.b.c`
- `this` resolution
- dynamic-view relationship-reference semantics
- every identifier-bearing DSL surface in one go

## Single-document boundary

This extraction pass is intentionally single-document.

It should produce facts from one parsed source file without requiring:

- workspace scanning
- include resolution
- cross-file symbol tables
- file watching

That means every extracted `Reference` in this note is an **observed syntax site**, not a resolved semantic binding.

The later workspace/index layer can match those references against bindable symbols.

## What counts as a symbol in the first pass

The first pass should treat a symbol as a declaration-like fact that is useful for at least one of:

- document outline structure
- later identifier resolution
- later navigation display

That immediately implies an important split.

## Structural symbols

A declaration may be structurally important even when it has no assigned identifier.

Examples:

- `person "User"`
- `softwareSystem "System"`
- `container "API"`
- `component "Worker"`

Those declarations should still become `Symbol` facts because:

- document symbols need them
- parent/child declaration structure needs them
- later UI surfaces may still want them by display name

## Bindable symbols

A declaration only becomes a later definition target when it carries a binding identifier.

Examples:

- `user = person "User"`
- `system = softwareSystem "System"`
- `api = container "API"`
- `worker = component "Worker"`
- `rel = user -> system "Uses"`

So the first-pass `Symbol` model should distinguish:

- a symbol's structural declaration kind
- its display name
- its optional binding name

That is cleaner than forcing every declaration into “definition or nothing”.

## Recommended public fact shape

The analysis crate skeleton already recommends owned facts.

For the first extraction pass, the key types should conceptually look like:

| Type                  | Role                                                                                                                             |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `Symbol`              | One declaration-like fact with kind, name, binding, span, and parent context                                                     |
| `Reference`           | One observed identifier reference site with kind, raw text, span, and container context                                          |
| `SymbolKind`          | Domain-shaped declaration kind (`Person`, `SoftwareSystem`, `Container`, `Component`, `Relationship`)                            |
| `ReferenceKind`       | Syntax role of the reference site (`RelationshipSource`, `RelationshipDestination`, `ViewScope`, `ViewInclude`, `ViewAnimation`) |
| `ReferenceTargetHint` | Narrow target-family hint (`Element`, `Deployment`, `Relationship`, `ElementOrRelationship`) where useful                        |

Important design choice:

- `Symbol` should be domain-shaped and optionally bindable
- `Reference` should be syntax-role-shaped and not yet resolved

## Minimum fields the first pass should carry

### `Symbol`

The first pass should capture at least:

- `kind`
- `display_name`
- `binding_name: Option<String>`
- `span`
- `parent_symbol: Option<...>`
- `syntax_node_kind` or equivalent debug-only trace

Useful interpretation:

- `display_name` comes from the declaration `name` field when present
- `binding_name` comes from the optional assignment identifier field
- `parent_symbol` captures structural nesting for outline/document-symbol consumers

### `Reference`

The first pass should capture at least:

- `kind`
- `raw_text`
- `span`
- `target_hint`
- containing symbol or containing syntax context

Important point:

- `raw_text` is what later resolution uses
- `target_hint` narrows later matching without pretending resolution already happened

## Supported symbol sites

The first pass should only emit symbols from syntax we have already audited and fixture-covered.

### Core declaration nodes

Emit `Symbol` facts for:

- `person`
- `software_system`
- `container`
- `component`

For each of these:

- `display_name` comes from the `name` field
- `binding_name` comes from the optional `identifier` field
- nesting is represented through `parent_symbol`

### Named relationships

Emit a `Symbol` fact for `relationship` nodes **only when** `relationship.identifier` is present.

Why:

- named relationships are the only relationship declarations that participate in later identifier navigation
- unnamed relationships are still useful for endpoint references, but not as bindable declaration targets

Recommended symbol shape for these:

- `kind = Relationship`
- `binding_name = Some(<relationship identifier>)`
- `display_name = description text when available, otherwise the binding name`

The exact display-label policy can change later, but the fact should exist.

## Supported reference sites

The first pass should extract only the syntax-simple reference sites we can explain clearly today.

### 1. Plain relationship endpoint identifiers

From `relationship` and `dynamic_relationship` nodes, emit `Reference` facts for:

- `source` when it is a plain `identifier`
- `destination` when it is a plain `identifier`

Use:

- `ReferenceKind::RelationshipSource`
- `ReferenceKind::RelationshipDestination`
- `ReferenceTargetHint::Element`

This covers:

- plain relationships like `user -> system`
- named relationships like `rel = user -> system`
- explicit dynamic-view edges like `web -> signin`

Do **not** emit these references when:

- the endpoint is `this_keyword`
- the `source` field is omitted

Those cases are explicitly deferred.

### 2. View scope identifiers

Emit `Reference` facts from the `scope` field of:

- `system_context_view`
- `container_view`
- `component_view`
- `deployment_view`
- `dynamic_view`

Use:

- `ReferenceKind::ViewScope`
- `ReferenceTargetHint::Element`

Why these are in scope:

- the syntax is direct and field-backed
- current fixtures already exercise them
- they are useful for navigation without pulling in broad runtime semantics

### 3. Identifier-valued view include statements

Emit `Reference` facts for `include_statement.value` when the value is a plain `identifier` inside these audited view blocks:

- `system_landscape_view`
- `system_context_view`
- `container_view`
- `component_view`
- `deployment_view`

Use:

- `ReferenceKind::ViewInclude`
- `ReferenceTargetHint::ElementOrRelationship` for non-deployment views
- `ReferenceTargetHint::Deployment` for `deployment_view`

This target hint is intentionally broader because a view include identifier may refer to:

- a model element identifier
- a named relationship identifier

Inside `deployment_view`, the same syntax instead targets deployment-layer
bindings such as deployment nodes and instances.

Do **not** treat:

- wildcards
- reluctant wildcards
- expressions
- quoted relationship expressions

as first-pass reference facts.

Those are later-phase view semantics, not bounded-MVP identifier extraction.

### 4. Identifier-valued view animation steps

Emit `Reference` facts for plain `identifier` values inside `animation_block` within:

- `system_landscape_view`
- `system_context_view`
- `container_view`
- `component_view`
- `deployment_view`

Use:

- `ReferenceKind::ViewAnimation`
- `ReferenceTargetHint::Element` for non-deployment views
- `ReferenceTargetHint::Deployment` for `deployment_view`

This keeps animation within the same direct-identifier navigation slice without
pulling dynamic-view relationship sequencing into the bounded MVP.

## Supported traversal contexts

The extraction walk should be narrow, but not too narrow.

## Declaration traversal

The first pass should walk declaration-bearing model contexts in pre-order:

- `model_block`
- `software_system_block`
- `container_block`

And it should descend through `group_block` **transparently** when needed to reach supported core declarations.

That is the recommended compromise:

- do not yet emit `group` as a first-pass symbol kind
- do not forget that grouped declarations are real declaration sites

This keeps the extractor useful for realistic grouped files without pretending group semantics are fully modeled.

## View traversal

The first pass should walk:

- `views_block`
- supported static view nodes listed above

Only the explicitly supported `scope`, identifier-valued `include_statement`, and
identifier-valued `animation_block` fields should emit reference facts.

## Directives in the same snapshot

Although this note is about symbol extraction, the first-pass snapshot should still include:

- `IncludeDirective` facts
- `IdentifierModeFact` facts

Those facts are not symbols or references themselves, but the analysis crate skeleton already treats them as part of the same bounded extraction slice.

## Explicitly deferred syntax

The first pass should intentionally skip these cases even when they appear syntactically simple at first glance.

### Relationship-related

- `this_keyword`
- `relationship` forms with omitted `source`
- `nested_relationship`
- `dynamic_relationship_reference`

### Selector/scope-related

- hierarchical selectors such as `a.b.c`
- `!element` / `!relationship` lookup targets
- view-key and tag-based references such as `filtered_view`

### Broader declaration surface

Do not yet emit first-pass symbols for:

- `group`
- `deployment_environment`
- `custom_element`
- `archetype_instance`

Those nodes are real declaration surfaces, but keeping them out of the first symbol-kind set helps preserve a clear bounded MVP.

Transparent traversal through `group_block` is enough for now.

## Parse-error policy

The extractor should **not** make symbol/reference extraction contingent on the entire tree being error-free.

Why:

- editors need useful partial structure while the user is typing
- Tree-sitter can still produce intact named nodes in many partially broken documents

Recommended rule:

- always extract syntax diagnostics
- attempt first-pass symbol/reference extraction from structurally recognized supported nodes
- carry the file's syntax-error state separately in the snapshot

Then later consumers can decide:

- document symbols may still be useful
- semantic diagnostics may need to be suppressed
- navigation should stay conservative when context is ambiguous

This matches the earlier diagnostics guidance better than “no symbols if any parse error exists”.

## Recommended extraction algorithm

The first implementation pass should stay explicit and handwritten.

### Phase A: collect directives

Extract:

- `IncludeDirective`
- `IdentifierModeFact`

from the contexts already documented in the directive audit.

### Phase B: walk model declarations

Walk supported model containers in document order.

For each supported declaration node:

1. emit a `Symbol`
1. push it onto the parent-symbol stack when it has a body that may contain supported declarations
1. recurse into supported child containers

For each `relationship` node:

1. emit a `Relationship` symbol only if `identifier` exists
1. emit endpoint reference facts only for plain identifier endpoints

### Phase C: walk supported view references

For each supported view node:

1. emit a `ViewScope` reference when `scope` is a plain identifier
1. emit `ViewInclude` references for identifier-valued include statements
1. emit `ViewAnimation` references for identifier-valued animation steps
1. for `dynamic_view`, emit `RelationshipSource` / `RelationshipDestination` references for plain-identifier `dynamic_relationship` endpoints

### Phase D: preserve pre-order output

The output order should match document pre-order as closely as possible.

Why:

- snapshot tests stay readable
- document symbols naturally preserve source order
- later parent/child relationships are easier to debug

## Fixture-backed expectations

The current fixture slice gives us a good minimum contract.

### [`crates/structurizr-lsp/tests/fixtures/identifiers/assigned-identifiers-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/identifiers/assigned-identifiers-ok.dsl)

Expected first-pass extraction:

- declaration symbols for `user`, `system`, `api`, `worker`, `platform`
- parent/child structure linking `system -> api -> worker`
- no references required from this fixture

### [`crates/structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl)

Expected first-pass extraction:

- declaration symbols for `user`, `system`, `api`, `worker`
- model relationship endpoint references from:
  - `user -> system`
  - `user -> api`
  - `user -> worker`
- view-scope references from:
  - `systemContext system`
  - `container system`
  - `component api`
- view-include identifier references from:
  - `include user`
  - `include api`
  - `include worker`

### [`crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl)

Expected first-pass extraction:

- declaration symbols for `user` and `system`
- a named `Relationship` symbol for `rel`
- relationship endpoint references for `user` and `system`
- a `ViewInclude` reference for `include rel`

### [`crates/structurizr-lsp/tests/fixtures/identifiers/dynamic-views-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/identifiers/dynamic-views-ok.dsl)

Expected first-pass extraction:

- declaration symbols for `system`, `web`, `api`, `signin`, `security`, and `database`
- a `ViewScope` reference for `dynamic api`
- `RelationshipSource` / `RelationshipDestination` references for:
  - `web -> signin`
  - `signin -> security`
  - `security -> database`

### [`crates/structurizr-lsp/tests/fixtures/relationships/named-relationship-dynamic-reference-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/relationships/named-relationship-dynamic-reference-ok.dsl)

Expected first-pass extraction:

- declaration symbols for `a` and `b`
- a named `Relationship` symbol for `rel`
- relationship endpoint references for `a` and `b`
- **no first-pass reference fact** for the `dynamic_relationship_reference`

That fixture should remain parse-covered but semantically deferred.

## How later consumers should use these facts

### Document symbols

Use the structural declaration `Symbol` tree.

That means:

- element declarations can appear even when they have no binding identifier
- relationship symbols can be filtered in or out by the LSP/UI later

### Go-to-definition

Only bindable symbols should be eligible definition targets:

- assigned core declaration identifiers
- named relationship identifiers

### Find references

Only the explicitly supported `ReferenceKind` values from this note should participate in the first bounded implementation.

Everything deferred should remain out of scope rather than partially guessed.

## Recommended implementation sequence

1. Define `Symbol`, `Reference`, `SymbolKind`, `ReferenceKind`, and `ReferenceTargetHint`.
1. Implement model-declaration walking for the supported declaration kinds.
1. Add relationship-symbol and endpoint-reference extraction.
1. Add supported view-scope and view-include reference extraction.
1. Snapshot the extracted facts against the current LSP fixtures.
1. Only then begin cross-file resolution or handler-level consumption.

That sequence keeps the first pass narrow and implementation-friendly.

## What this unblocks

Once this extraction contract exists, the next implementation steps have a much clearer target:

- the analysis crate can expose a stable first-pass fact surface
- the future LSP handlers can consume those facts instead of re-walking syntax trees
- the bounded definition/reference story becomes concrete enough to test against realistic fixtures

## Sources

- [`docs/lsp/02-design/analysis-crate-skeleton.md`](analysis-crate-skeleton.md)
- [`docs/lsp/90-history/syntax-audit-assignment-declarations.md`](../90-history/syntax-audit-assignment-declarations.md)
- [`docs/lsp/90-history/syntax-audit-reference-relationship-nodes.md`](../90-history/syntax-audit-reference-relationship-nodes.md)
- [`docs/lsp/90-history/syntax-audit-directive-nodes.md`](../90-history/syntax-audit-directive-nodes.md)
- [`docs/lsp/90-history/phase1-backlog.md`](../90-history/phase1-backlog.md)
- [`crates/structurizr-lsp/tests/fixtures/identifiers/assigned-identifiers-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/identifiers/assigned-identifiers-ok.dsl)
- [`crates/structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl)
- [`crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/relationships/named-relationships-ok.dsl)
- [`crates/structurizr-lsp/tests/fixtures/relationships/named-relationship-dynamic-reference-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/relationships/named-relationship-dynamic-reference-ok.dsl)
- corresponding snapshots under [`crates/structurizr-grammar/tests/snapshots/`](../../../crates/structurizr-grammar/tests/snapshots/)
