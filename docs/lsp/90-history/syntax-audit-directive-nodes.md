# Structurizr DSL syntax audit: directive nodes

This note records the parse-tree contract for the two directive shapes that matter most to the bounded MVP planning work:

- `!include`
- `!identifiers`

The goal is to make later analysis work precise about:

- where these directives can appear
- what node shape they produce
- what data can be extracted safely at syntax time

It does **not** attempt to define include resolution, variable substitution, or the final semantics of identifier modes.

## Main conclusion

Both directives are intentionally modeled as lightweight syntax forms.

That is good for editor tooling:

- they are easy to detect
- they expose a single named `value` field
- they can be extracted without walking complicated subtrees

But it also means the analyzer should be conservative:

- record the raw directive value and its node kind
- record where the directive appears
- defer path resolution, substitution, and semantic validation to later layers

However, review against the published DSL reference suggests that these two directives should not be treated as having identical placement semantics:

- `!identifiers` is clearly documented as workspace/model scoped
- `!include` behaves more like an inlining directive, and the current grammar may actually be narrower than upstream in where it accepts it

## Shared value shape

Both directives use the same grammar helper:

```js
_directive_value: ($) =>
  choice($.string, $.text_block_string, $.bare_value, $.identifier))
```

That means both `include_directive.value` and `identifiers_directive.value` can be any of:

- `bare_value`
- `identifier`
- `string`
- `text_block_string`

This is broader than the values the future semantic layer will probably want to treat as valid.

### Why this matters

- `!include` does not syntactically enforce “path-like” values
- `!identifiers` does not syntactically enforce a fixed enum like `flat` or `hierarchical`
- the analyzer should not hard-code assumptions from current fixtures into the syntax layer

## `include_directive`

Grammar rule:

```js
include_directive: ($) =>
  seq("!include", field("value", $._directive_value))
```

Node kind:

- `include_directive`

Fields from `node-types.json`:

- `value` — required

Allowed child types for `value`:

- `bare_value`
- `identifier`
- `string`
- `text_block_string`

### What current fixtures show

Current local fixtures and corpus examples show `!include` being used with:

- relative-looking paths
- directory-looking values
- full URL-looking values

Example fixture values:

- `include/model/software-system/model.dsl`
- `include/model/software-system`
- `https://raw.githubusercontent.com/.../model.dsl`

In the existing corpus example, `!include include/model.dsl` parses as:

```text
(include_directive
  (bare_value))
```

### Syntax-time guidance

The analyzer can safely extract:

- the existence of an include directive
- the raw `value` child node
- the source range / raw text of that value
- the container in which it appears

The analyzer should **not** assume at syntax time:

- the include target exists
- the include target is local rather than remote
- the include target is a file rather than a directory-like token
- the include value has already been expanded through `!const` or `!var`

## `identifiers_directive`

Grammar rule:

```js
identifiers_directive: ($) =>
  seq("!identifiers", field("value", $._directive_value))
```

Node kind:

- `identifiers_directive`

Fields from `node-types.json`:

- `value` — required

Allowed child types for `value`:

- `bare_value`
- `identifier`
- `string`
- `text_block_string`

### What current fixtures show

Current fixtures and corpus examples use values like:

- `hierarchical`
- `flat`

Those values currently parse as identifier-like directive values rather than a dedicated enum node.

Example parse shape from the corpus:

```text
(identifiers_directive
  (identifier))
```

### Syntax-time guidance

The analyzer can safely extract:

- that an identifiers directive exists
- the raw directive value
- the scope/container in which it was declared

The analyzer should **not** assume at syntax time:

- only `flat` and `hierarchical` will ever appear in parsed trees
- the directive value has already been semantically validated
- the directive by itself settles rename/completion behavior without a later policy decision

## Where these directives currently appear in the grammar

This is the most important structural finding for future analysis, but it needs to be interpreted carefully.

Today the grammar allows both directives in more than one place:

- directly at `source_file` root
- inside `workspace_block`
- inside `model_block`

### `source_file`

`source_file` may contain both:

- `include_directive`
- `identifiers_directive`

For the parser, this means the future analyzer cannot limit directive scanning to `workspace` or `model` nodes only.

But for language-reference alignment, this should be treated as **current grammar behavior**, not automatically as canonical DSL scope.

### `workspace_block`

`workspace_block` also allows:

- `include_directive`
- `identifiers_directive`

This is the common case for workspace-wide identifier mode and workspace-level includes.

### `model_block`

`model_block` also allows:

- `include_directive`
- `identifiers_directive`

This matters because model-scoped directives may need to be recorded separately from workspace-scoped ones in later semantic work.

## Placement nuance: `!identifiers` versus `!include`

The published DSL reference gives these directives different signals.

## `!identifiers`

The language reference documents `!identifiers` as a workspace/model concern.

That means:

- `workspace_block` and `model_block` support are reference-backed
- `source_file` root support should be treated as parser convenience for fragments unless proven otherwise
- the analyzer should not assume that root-level `!identifiers` is canonical DSL scope

## `!include`

The language reference documents `!include <file|directory|url>`, but it is less explicit about all syntactic placement contexts than it is for `!identifiers`.

That means:

- current root/workspace/model support is enough for local coverage
- but the current grammar may still be too narrow if upstream effectively treats includes as more general inlining directives
- this is a grammar-alignment question worth keeping open rather than freezing into the audit as settled fact

## Do not confuse directive nodes with view include/exclude statements

This audit is about:

- `include_directive`
- `identifiers_directive`

It is **not** about:

- `include_statement`
- `exclude_statement`

Those appear in view blocks and have different syntax and semantics.

It is also worth noting that `views_block` does **not** currently accept `identifiers_directive`, so view scopes should not be treated as another identifier-mode declaration context.

Important distinction:

- `include_directive` has a single `value` field
- `include_statement` can carry repeated values and belongs to view-level include/exclude behavior

Future analysis should keep these concerns separate.

## Parse-shape examples

## Example 1: model-level include directive

From the corpus:

```dsl
model {
  !include include/model.dsl
}
```

Parse shape:

```text
(include_directive
  (bare_value))
```

This confirms the directive is a very small node with a single child carrying the raw value.

## Example 2: workspace-level identifiers directive

From the corpus:

```dsl
workspace {
  !identifiers hierarchical
}
```

Parse shape:

```text
(identifiers_directive
  (identifier))
```

This confirms the directive value is not a specialized enum node.

## Analyzer guidance for the bounded MVP

The first semantic layer should record directive facts like:

- directive kind (`include` vs `identifiers`)
- containing scope (`source_file`, `workspace_block`, or `model_block`)
- raw value node kind
- raw value text
- source range

That is enough to support later work on:

- include resolution
- diagnostics for missing/cyclic includes
- identifier-mode decisions for completion and rename

Without prematurely deciding semantics in the syntax pass.

## Current verdict

There is **no obvious syntax blocker** for extracting `!include` and `!identifiers` facts.

The main future decisions are semantic:

- how include targets should be resolved
- whether and how `!const` / `!var` affect include values
- how `!identifiers` should influence rename and completion behavior
- how precedence should work if directives appear at different container levels

There are also two grammar-alignment questions worth carrying forward:

- whether root-level `identifiers_directive` should remain as parser convenience or be narrowed to match the DSL reference more closely
- whether `include_directive` should eventually be allowed in more contexts if upstream include handling proves broader than current grammar placement

## Sources

- [`crates/strz-grammar/grammar.js`](../../../crates/strz-grammar/grammar.js)
- [`crates/strz-grammar/src/node-types.json`](../../../crates/strz-grammar/src/node-types.json)
- [`crates/strz-grammar/test/corpus/workspace.txt`](../../../crates/strz-grammar/test/corpus/workspace.txt)
- [`crates/strz-grammar/test/corpus/model.txt`](../../../crates/strz-grammar/test/corpus/model.txt)
- [`fixtures/workspace/extension_directives-ok.dsl`](../../../fixtures/workspace/extension_directives-ok.dsl)
- [`fixtures/views/advanced-ok.dsl`](../../../fixtures/views/advanced-ok.dsl)
