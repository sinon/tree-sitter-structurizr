# Structurizr DSL syntax audit: assignment and declaration nodes

This note records the parse-tree contract for the first bounded-MVP analysis surface.

It is intentionally narrow. It covers:

- top-level identifier assignment embedded in declarations
- direct model-element declarations
- the declaration contexts those nodes can appear in

It does **not** try to settle broader reference or scope behavior yet.

## Main conclusion

For the core Structurizr model elements, assignment is **not** represented as a separate AST node.

Instead, assignment is embedded directly inside declaration nodes via an optional `identifier` field followed by `"="`.

That means the future analysis crate should treat these nodes as the canonical definition sites:

- `person`
- `software_system`
- `container`
- `component`

The analyzer should **not** expect a standalone “assignment statement” wrapper node for these cases.

## Shared declaration pattern

The core element rules in `crates/structurizr-grammar/grammar.js` all follow the same structure:

1. optional identifier assignment
2. element keyword
3. required `name`
4. optional metadata fields
5. optional body block

Representative grammar shape:

- `person` uses `optional(seq(field("identifier", $._assignment_identifier), "="))`
- `software_system` uses the same embedded assignment shape
- `container` uses the same embedded assignment shape
- `component` uses the same embedded assignment shape

The same embedded assignment pattern is also reused by extension-point declarations such as:

- `group`
- `deployment_environment`
- `custom_element`
- `archetype_instance`

That reuse is useful context, but the bounded MVP can still focus first on the four core model-element declarations above while remembering that grouped and deployment contexts also expose assignment-bearing nodes.

## Assignment identifier shape

The optional assigned identifier comes from `_assignment_identifier`, not directly from the plain `identifier` rule.

Important detail:

- `_assignment_identifier` accepts plain identifiers
- it also aliases several Structurizr keywords to `identifier`

Examples include aliases for:

- `person`
- `softwareSystem`
- `container`
- `component`
- deployment-related keywords

Why this matters:

- a source file may use names that would otherwise collide with DSL keywords
- in the final tree, the assignment slot still appears as an `identifier` node
- the analyzer can rely on the `identifier` field in node-types, not on token-text heuristics

## Core declaration nodes and fields

## `person`

Node kind:

- `person`

Fields from `node-types.json`:

- `identifier` — optional definition identifier
- `name` — required
- `description` — optional
- `tags` — optional
- `body` — optional `person_block`

Notes:

- `name` can be either `identifier` or `string`
- if present, the embedded assignment identifier is the stable definition key for later symbol extraction
- body contents are additional structure, not part of the identifier-assignment shape itself

## `software_system`

Node kind:

- `software_system`

Fields from `node-types.json`:

- `identifier` — optional definition identifier
- `name` — required
- `description` — optional
- `tags` — optional
- `body` — optional `software_system_block`

Notes:

- the keyword surface accepts both `softwareSystem` and `softwaresystem`
- the resulting node kind is still `software_system`
- nested `container` declarations appear inside `software_system_block`

## `container`

Node kind:

- `container`

Fields from `node-types.json`:

- `identifier` — optional definition identifier
- `name` — required
- `description` — optional
- `technology` — optional
- `tags` — optional
- `body` — optional `container_block`

Notes:

- `container` is the first core declaration kind where `technology` appears as a stable optional field
- nested `component` declarations appear inside `container_block`

## `component`

Node kind:

- `component`

Fields from `node-types.json`:

- `identifier` — optional definition identifier
- `name` — required
- `description` — optional
- `technology` — optional
- `tags` — optional
- `body` — optional `component_block`

Notes:

- this has the same assignment shape as the higher-level declarations
- for bounded MVP analysis, the main thing to rely on is still the optional `identifier` field

## Other assignment-enabled declaration nodes worth tracking

These are not part of the smallest bounded-MVP definition slice, but the grammar already gives them the same embedded assignment shape:

- `group`
- `deployment_environment`
- `custom_element`
- `archetype_instance`

This matters because a future analyzer should not assume that only the four core model-element declarations can introduce assignment-backed definition sites.

## Declaration contexts the analyzer should care about first

The future analyzer does not just need node kinds; it also needs to know where to look for them.

## `model_block`

`model_block` is the broadest declaration container for bounded-MVP work.

Relevant child node kinds include:

- `person`
- `software_system`
- `group`
- `deployment_environment`
- `custom_element`
- `archetype_instance`
- `include_directive`
- `identifiers_directive`
- `relationship`

For bounded MVP analysis, this means:

- top-level symbol extraction should begin from `model_block`
- the core model-element definition sites available directly here are `person` and `software_system`
- `group` and `deployment_environment` are also assignment-enabled declaration sites here
- nested `container` and `component` definitions should be found by descending into `software_system_block` and then `container_block`

## `software_system_block`

Relevant child node kinds include:

- `group`
- `container`
- `custom_element`
- `archetype_instance`
- `relationship`

For bounded MVP analysis:

- nested container definitions should be expected here
- this is the next context after `model_block` that symbol extraction will likely need

## `container_block`

Relevant child node kinds include:

- `group`
- `component`
- `custom_element`
- `archetype_instance`
- `relationship`

For bounded MVP analysis:

- nested component definitions should be expected here

## `group_block`

`group_block` is an important missing traversal context if the analyzer wants to handle realistic grouped declarations.

The corpus already exercises assigned nested groups and grouped declarations, so later analysis should not assume that all definition sites are reachable only through `model_block`, `software_system_block`, and `container_block`.

For bounded MVP analysis:

- grouped declarations should be treated as real definition sites once grouped contexts are brought into scope
- the first LSP slice can still defer full group semantics, but it should not forget that grouped declarations already exist in the syntax surface

## Parse-tree examples

## Example 1: assigned top-level declarations

From the corpus:

```dsl
workspace {
  model {
    user = person "User"
    system = softwareSystem "System" {
      api = container "API" "Handles requests" "Rust" {
        worker = component "Worker" "Processes jobs" "Rust"
      }
    }
  }
}
```

Corresponding parse shape:

```text
(person
  (identifier)
  (string))
(software_system
  (identifier)
  (string)
  (software_system_block
    (container
      (identifier)
      (string)
      (string)
      (string)
      (container_block
        (component
          (identifier)
          (string)
          (string)
          (string))))))
```

This confirms:

- assigned identifiers are direct children of the declaration node
- there is no separate assignment wrapper node
- nested declarations keep the same declaration shape

## Example 2: workspace-level directive plus assigned declarations

From the corpus:

```dsl
workspace {
  !identifiers hierarchical

  model {
    user = person "User"
    system = softwareSystem "System"
  }
}
```

Relevant parse shape:

```text
(identifiers_directive
  (identifier))
(person
  (identifier)
  (string))
(software_system
  (identifier)
  (string))
```

This is useful because later semantic work around `!identifiers` can be layered on top of the same declaration shapes without changing where definitions live.

## Analyzer guidance for the bounded MVP

The first analysis pass should treat the following as stable definition sites:

- `person.identifier`
- `software_system.identifier`
- `container.identifier`
- `component.identifier`

It should also record, even if later semantic support is phased in more cautiously:

- `group.identifier`
- `deployment_environment.identifier`

Recommended first-pass extraction behavior:

1. walk the allowed declaration containers
2. collect declaration nodes of the supported kinds
3. read the optional `identifier` field when present
4. store `name` separately from the identifier because they are not the same DSL concept

The analyzer should **not** assume yet that every declaration node with a body is always a valid symbol target for the bounded MVP. Scope and support rules will be tightened in the later scope-rules audit.

## Known implications for future work

- later audits should cover direct reference nodes separately from declaration nodes
- later audits should cover directive node shapes such as `!include` and `!identifiers` in more detail
- later scope work must decide how much of nested declaration structure is supported in the first go-to-definition and reference implementation
- grammar/reference alignment still needs review for some declaration-body children, notably `!components` in `container` bodies and `!docs` / `!adrs` in `component` bodies

## Current verdict

There is **no obvious syntax blocker** for extracting bounded-MVP definitions from core declaration nodes.

The main follow-up questions are semantic rather than syntactic:

- which declaration contexts count as supported definition sites in MVP
- how nested scopes should be handled later
- how `!identifiers` should affect completion and rename behavior

## Sources

- `crates/structurizr-grammar/grammar.js`
- `crates/structurizr-grammar/src/node-types.json`
- `crates/structurizr-grammar/test/corpus/model.txt`
- `crates/structurizr-grammar/test/corpus/workspace.txt`
- `crates/structurizr-grammar/tests/fixtures/model/nested_elements-ok.dsl`
