# Structurizr DSL syntax audit: reference and relationship nodes

This note records the parse-tree contract for the bounded-MVP reference surface.

It focuses on:

- direct identifier-bearing relationship nodes
- relationship assignment identifiers
- view nodes that carry simple identifier references
- the node shapes that should be deferred because they require broader scope semantics

It does **not** attempt to settle final resolution rules yet. It is a syntax audit, not a semantic policy.

## Main conclusion

The grammar exposes a useful bounded reference surface, but it does not fully separate all of the easy and hard cases into distinct node kinds.

For bounded MVP analysis, the safest syntax contract is:

- support plain `relationship` nodes that use identifier endpoints
- support relationship assignment identifiers on those nodes
- support simple view-level identifier references carried in obvious fields like `scope`
- explicitly defer `this`, nested shorthand relationships, and dynamic relationship references

That gives a workable first slice for definition/reference features without pretending the harder scope questions are solved.

## Relationship nodes

## `relationship`

Node kind:

- `relationship`

Important fields from `node-types.json`:

- `identifier` — optional named relationship identifier
- `source` — optional, `identifier` or `this_keyword`
- `operator` — required `relationship_operator`
- `destination` — required, `identifier` or `this_keyword`
- `attribute` — optional, repeated metadata values
- `body` — optional `relationship_block`

Important grammar facts:

- the rule supports assigned relationships like `rel = a -> b`
- the rule supports plain relationships like `a -> b`
- the rule supports shorthand forms with omitted source like `-> database`
- the rule allows `this` via `_relationship_endpoint`
- the operator can be plain (`->`, `-/>`) or archetyped (`--https->`)

Important type detail:

- `relationship.attribute` can be either `identifier` or `string`
- this is looser than `nested_relationship.attribute`, which is string-only

### Bounded-MVP guidance

Treat these as in-scope first:

- `relationship.identifier` when present
- `relationship.source` when it is a plain `identifier`
- `relationship.destination` when it is a plain `identifier`

Treat these as deferred:

- any `relationship.source` or `relationship.destination` using `this_keyword`
- shorthand forms where `source` is omitted
- semantics of archetyped operators beyond preserving the operator text

One subtlety: shorthand forms with omitted source are not confined to one special node kind. They are represented by the same `relationship` node with an absent `source` field, so the analyzer must check field presence rather than node kind.

## `nested_relationship`

Node kind:

- `nested_relationship`

Important fields:

- `source` — optional, `identifier` or `this_keyword`
- `operator` — required
- `destination` — required, `identifier` or `this_keyword`
- `attribute` — optional metadata values

Why it matters:

- `nested_relationship` appears inside `relationship_block`
- it shares the same endpoint ambiguity as `relationship`
- it also allows omitted `source`
- unlike `relationship`, it has no assignment `identifier` field of its own

### Bounded-MVP guidance

Defer `nested_relationship` entirely.

Reason:

- it combines nesting, optional source, and possible `this_keyword`
- supporting it early would blur the MVP boundary that is supposed to stay “obvious direct references only”

## `dynamic_relationship`

Node kind:

- `dynamic_relationship`

Important fields:

- `order` — optional
- `source` — required `identifier`
- `destination` — required `identifier`
- `description` — optional
- `technology` — optional
- `body` — optional `dynamic_relationship_block`

Why it matters:

- this is structurally cleaner than plain `relationship` because `source` and `destination` are always identifiers
- but it lives in dynamic-view semantics, which are broader than the bounded MVP

### Bounded-MVP guidance

Defer for the first implementation slice, even though the syntax is relatively clean.

Reason:

- the roadmap already excludes dynamic-view relationship work from the bounded MVP
- keeping it deferred avoids mixing ordinary model references with dynamic-view sequencing semantics

## `dynamic_relationship_reference`

Node kind:

- `dynamic_relationship_reference`

Important fields:

- `relationship` — required `identifier`
- `description` — required in the current grammar
- `order` — optional

Why it matters:

- this is a direct reference to a previously named relationship
- syntax is simple, but semantics depend on relationship naming and dynamic-view resolution
- the current grammar is stricter than the DSL reference here, because the published syntax allows `[description]` rather than requiring one

### Bounded-MVP guidance

Defer.

Reason:

- this is exactly the kind of “looks simple but pulls in broader scope rules” case the MVP should avoid
- it depends on relationship naming and view context, not just raw identifier lookup

Reference-alignment note:

- making `dynamic_relationship_reference.description` optional would better match the DSL reference and would not make the syntax harder for the LSP to consume

## `this_keyword`

Node kind:

- `this_keyword`

Where it appears in the audited surface:

- `relationship.source`
- `relationship.destination`
- `nested_relationship.source`
- `nested_relationship.destination`

Corpus evidence shows it also appears inside deployment instance bodies and similar nested contexts.

### Bounded-MVP guidance

Defer all `this_keyword` resolution.

Reason:

- `this` is inherently scope-sensitive
- its meaning depends on the containing element, instance, or nested body
- the roadmap already identifies it as a deferred case

## View nodes carrying direct references

## `container_view`

Node kind:

- `container_view`

Important fields:

- `scope` — required `identifier`
- `key` — optional
- `description` — optional
- `body` — required

### Bounded-MVP guidance

The `scope` field is a clean identifier-bearing field and is a good candidate for future simple reference extraction.

However:

- view resolution itself is not required for the first bounded definition/reference pass
- it should be recorded as “syntax is clean; semantics can come later”

## `component_view`

Node kind:

- `component_view`

Important fields:

- `scope` — required `identifier`
- `key` — optional
- `description` — optional
- `body` — required

### Bounded-MVP guidance

Same guidance as `container_view`.

The syntax is straightforward, but there is no need to pull it into the first definition/reference implementation unless the MVP grows intentionally.

## `filtered_view`

Node kind:

- `filtered_view`

Important fields:

- `base_key`
- `mode`
- `tags`
- `key` — optional
- `description` — optional
- `body` — optional

### Bounded-MVP guidance

Treat `filtered_view` as out of scope for early reference analysis.

Reason:

- it references view keys and tags rather than simple model identifiers
- it is a later-phase navigation/hover concern, not part of the first direct identifier slice

## Parse-shape examples

## Example 1: named relationship in the model

From the corpus:

```dsl
model {
  a = softwareSystem "A"
  b = softwareSystem "B"
  rel = a -> b
}
```

Relevant parse shape:

```text
(relationship
  (identifier)
  (identifier)
  (relationship_operator)
  (identifier))
```

This confirms:

- named relationships are still `relationship` nodes
- the relationship name is just the optional `identifier` field on the node
- source and destination are plain identifiers in the simple case

## Example 2: dynamic relationship reference

From the corpus:

```dsl
model {
  rel = a -> b "Async"
}

views {
  dynamic * {
    rel "Async"
  }
}
```

Relevant parse shape:

```text
(dynamic_relationship_reference
  (identifier)
  (string))
```

This confirms:

- relationship references inside dynamic views have a dedicated node kind
- syntax is easy to recognize, but semantic resolution is still broader than the bounded MVP
- the current grammar requires a description in this form, which is stricter than the DSL reference

## Example 3: `this` in nested/deployment contexts

From the corpus:

```dsl
infra -> this
```

Relevant parse shape:

```text
(relationship
  (identifier)
  (relationship_operator)
  (this_keyword))
```

This confirms:

- `this` is not text hidden inside an identifier node
- the tree exposes it distinctly as `this_keyword`
- the analyzer can cleanly defer it without ambiguous token inspection

## Bounded-MVP syntax contract

For the first implementation slice, the analysis layer can safely treat the following as in-scope reference-bearing syntax:

- `relationship.identifier`
- `relationship.source` when it is an `identifier`
- `relationship.destination` when it is an `identifier`

It can also record, but not yet semantically resolve:

- `container_view.scope`
- `component_view.scope`

The following should be marked deferred in the analysis design:

- `this_keyword`
- `nested_relationship`
- shorthand relationships with omitted source
- `dynamic_relationship`
- `dynamic_relationship_reference`
- `filtered_view` key/tag semantics

## Grammar follow-ups surfaced by review

Worth doing soon:

- make `dynamic_relationship_reference.description` optional to align with the DSL reference
- consider naming static relationship metadata more explicitly in the tree if future LSP work needs better hover/completion support than a generic repeated `attribute` field provides

Reasonable to defer:

- splitting shorthand and explicit static relationships into separate node kinds
- any attempt to normalize omitted source into synthetic `this`
- extra churn in view node shapes that are already clean enough for later semantic work

## Current verdict

There is **no obvious syntax blocker** for bounded reference extraction over direct model relationships and named relationships.

The main boundary is not parser capability; it is discipline about what the first semantic layer chooses to support, plus one concrete grammar-alignment fix around `dynamic_relationship_reference.description`.

## Sources

- `crates/structurizr-grammar/grammar.js`
- `crates/structurizr-grammar/src/node-types.json`
- `crates/structurizr-grammar/test/corpus/workspace.txt`
- `crates/structurizr-grammar/test/corpus/views.txt`
- `crates/structurizr-grammar/test/corpus/model.txt`
- `crates/structurizr-grammar/tests/fixtures/views/dynamic-explicit-relationships-ok.dsl`
- `crates/structurizr-grammar/tests/fixtures/deployment/instance_bodies-ok.dsl`
