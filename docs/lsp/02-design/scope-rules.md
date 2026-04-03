# Structurizr DSL bounded scope rules

> Status: implemented in bounded form.
>
> The bounded resolution rules described here are now active in the in-repo
> analysis and LSP layers. Read remaining future-tense wording below as the
> original rationale for those limits and follow-on expansion notes.

This note defines the first supported identifier-resolution rules for the future Structurizr analysis crate and LSP.

It is the semantic companion to:

- [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md)
- [`docs/lsp/02-design/first-pass-symbol-extraction.md`](first-pass-symbol-extraction.md)
- [`docs/lsp/02-design/bounded-mvp-handlers.md`](bounded-mvp-handlers.md)

The goal is to make the bounded MVP precise about what resolves now, what stays deferred, and how `!identifiers` should shape later completion and rename behavior.

## Why this note exists

The syntax audits and extraction notes tell us:

- where definition-like nodes live
- where simple reference sites live
- which syntax is already intentionally deferred

But they do not, by themselves, answer the next question:

- when a bindable identifier exists, what name does it resolve under?

That is where scope rules begin.

## Main conclusion

The bounded MVP should resolve identifiers against a **workspace instance** using a **canonical binding key** model.

The first key rules are:

1. element and relationship bindings are separate semantic tables
1. element bindings are affected by the effective `!identifiers` mode
1. relationship bindings are **not** affected by `!identifiers`
1. groups do not contribute hierarchical path segments
1. selector-based reference syntax remains deferred even if canonical hierarchical keys are computed internally

This gives the project a stable semantic model without forcing the first handler slice to solve every reference surface immediately.

## Resolution unit: workspace instance

The workspace-discovery note already established the right semantic unit:

- one entry document
- plus its transitive local include closure

That same rule should govern identifier resolution.

The server should **not** resolve identifiers against one folder-wide symbol bag.

Instead, every semantic answer should be evaluated against one workspace instance:

- root document
- included fragments reachable from that root
- one effective set of bindable symbols for that instance

### Open fragments

If an open fragment belongs to:

- **zero** known workspace instances: syntax-only behavior
- **one** known workspace instance: full bounded semantic behavior
- **multiple** known workspace instances: only return a semantic answer when it is identical across all candidate contexts

That keeps scope-sensitive features conservative instead of pretending a fragment has one universal semantic owner.

## Symbol classes and binding tables

The first-pass extraction note distinguishes structural symbols from bindable symbols.

The scope rules note adds the next distinction:

- **element bindings**
- **relationship bindings**

These should be tracked in separate semantic tables.

### Element binding table

Contains bindable assigned identifiers from supported core declarations:

- `person`
- `software_system`
- `container`
- `component`

Only declarations with an assignment identifier contribute keys to this table.

### Relationship binding table

Contains bindable named relationships:

- `relationship.identifier`

Only relationships with an assigned identifier contribute keys to this table.

### Why split the tables

The current planning work already knows that:

- view include references may point at either elements or relationships
- relationship identifiers are unaffected by `!identifiers`

So separate tables are safer than assuming one shared namespace policy that we have not yet formally documented.

For bounded MVP navigation:

- relationship endpoints and view scopes query the element table only
- named-relationship references query the relationship table
- view include identifiers may query both, with ambiguity handled conservatively

## Effective `!identifiers` mode

The public Structurizr identifiers docs give the key semantic rule:

- default mode is `flat`
- `!identifiers hierarchical` changes **element** identifier scoping
- relationship identifiers are unaffected
- hierarchical mode does **not** apply to groups

The future analyzer should therefore compute an effective element-identifier mode for each supported declaration context.

## Default

If no reference-backed directive overrides it, the effective mode is:

- `flat`

## Supported directive scopes for semantics

For bounded MVP, only these directive locations should participate in effective-mode computation:

- `workspace_block`
- `model_block`

That matches the language reference more closely than treating root-level directives as canonical.

### Root-level directive nuance

The grammar currently permits `identifiers_directive` at `source_file` root.

For bounded MVP semantics, treat that as:

- parsed and recorded
- useful for fragment/debug visibility
- **not** canonical for identifier-mode computation unless later design work explicitly broadens the rule

This keeps parser convenience separate from semantic policy.

## Precedence rule

Use nearest applicable directive wins:

1. model-level `!identifiers` overrides workspace-level `!identifiers`
1. workspace-level `!identifiers` applies when no model-level override exists
1. otherwise default to `flat`

This is the cleanest bounded rule and fits the current grammar/reference split better than treating all observed directives as equal.

## Canonical binding keys

The bounded MVP should compute a canonical binding key for every bindable symbol.

This key is the thing later resolution matches against.

### Flat mode

In `flat` mode:

- canonical element key = local assignment identifier text

Examples:

- `user = person "User"` -> `user`
- `system = softwareSystem "System"` -> `system`
- `api = container "API"` -> `api`
- `worker = component "Worker"` -> `worker`

This is the default global-scoping behavior described by the Structurizr docs.

### Hierarchical mode

In `hierarchical` mode:

- top-level model element keys remain their local assignment identifiers
- nested element keys are formed by joining ancestor element binding keys with `.`
- groups do **not** contribute path segments

Examples:

- `system = softwareSystem "System"` -> `system`
- nested `api = container "API"` -> `system.api`
- nested `worker = component "Worker"` -> `system.api.worker`

### Important constraint

A nested element only gets a canonical hierarchical key when every required ancestor in the supported declaration chain has a bindable element key.

That means:

- if a parent declaration has no assignment identifier, its nested bindable children do **not** gain a stable hierarchical key for bounded MVP resolution

This is conservative, but it is much safer than fabricating unstable anonymous path segments from display names.

### Relationship identifiers

Relationship identifiers are always flat in the bounded MVP:

- canonical relationship key = local relationship assignment identifier text

This is true regardless of `!identifiers`.

Example:

- `rel = user -> system "Uses"` -> `rel`

## Groups are transparent to key construction

The Structurizr identifiers docs explicitly say that `!identifiers hierarchical` does not apply to groups.

The bounded MVP should therefore treat groups as transparent for canonical key construction.

That means:

- groups may affect where declarations appear in the tree
- they do **not** add identifier path segments

This aligns cleanly with the earlier recommendation to traverse `group_block` transparently in first-pass symbol extraction.

## What resolves in the bounded MVP

The first handler slice only supports a subset of extracted reference kinds.

The scope rules for those kinds should be:

### 1. Relationship endpoint references

For:

- `ReferenceKind::RelationshipSource`
- `ReferenceKind::RelationshipDestination`

resolve against:

- the element binding table only

using:

- exact canonical key match

This means:

- in flat mode, `user -> system` resolves through `user` and `system`
- explicit dynamic-view edges such as `web -> signin` resolve through the same element binding table
- in hierarchical mode, a simple `api` reference does **not** resolve to `system.api`

That outcome is intentional until selector reference syntax is in scope.

### 2. View scope references

For:

- `ReferenceKind::ViewScope`

resolve against:

- the element binding table only

using:

- exact canonical key match

This covers:

- `systemContext system`
- `container system`
- `component api` in flat mode
- `deployment system "Live"`
- `dynamic api "SignIn"`

It does **not** automatically make hierarchical shorthand work.

### 3. View include references

For:

- `ReferenceKind::ViewInclude`

attempt resolution against:

1. the element binding table when the reference carries `ReferenceTargetHint::ElementOrRelationship`
1. the relationship binding table when the reference carries `ReferenceTargetHint::ElementOrRelationship`
1. the deployment binding table when the reference carries `ReferenceTargetHint::Deployment`

using:

- exact canonical key match in each table

Conservative rule:

- if exactly one target is found, return it
- if zero targets are found, return no result
- if both tables produce a candidate for the same raw text, treat the result as ambiguous and return no result until namespace policy is documented more fully

This avoids silently choosing the wrong target kind.

### 4. View animation references

For:

- `ReferenceKind::ViewAnimation`

resolve against:

- the element binding table when the reference carries `ReferenceTargetHint::Element`
- the deployment binding table when the reference carries `ReferenceTargetHint::Deployment`

using:

- exact canonical key match in the relevant table

This covers:

- identifier steps inside `animation { ... }` for `systemLandscape`, `systemContext`, `container`, and `component` views
- deployment-node and instance identifiers inside `deployment` view animations

It does **not** widen support to named `dynamic_relationship_reference` sites.

## What stays explicitly deferred

The bounded MVP should **not** resolve:

- `this`
- omitted-source relationship shorthand
- `nested_relationship`
- `dynamic_relationship_reference`
- hierarchical selector reference syntax such as `system.api`
- `!element` / `!relationship` selector lookups
- filtered-view key/tag references
- any broader runtime-style naming behavior

Computing hierarchical canonical keys does **not** mean selector reference syntax becomes supported automatically.

That is an important distinction:

- key construction can be ready before selector reference extraction is ready

## Why selector syntax is still deferred

This is the most subtle design choice in the note.

The project should compute hierarchical canonical keys now because:

- it stabilizes the internal index model
- it gives future identifier completion a target shape
- it avoids redesigning element keys later

But the first extraction/handler slice should still defer selector **reference sites** because:

- they are not part of the audited first-pass reference set
- they introduce more parsing/resolution surfaces at once
- they complicate rename/edit planning immediately

So the bounded MVP should separate:

- canonical hierarchical key computation: **supported internally**
- selector reference syntax handling: **deferred externally**

## Parse-error interaction

The first-pass extraction note already says symbol/reference extraction should continue where supported nodes remain structurally intact.

Scope resolution should follow the same spirit:

- if a supported reference fact exists, attempt bounded resolution
- if surrounding syntax makes the result ambiguous, return no confident answer

This means parse errors should not automatically disable:

- document symbols
- all definition results
- all reference results

But they **should** keep later semantic diagnostics conservative.

## `!identifiers` and completion

The bounded handler note already keeps the first completion slice keyword/directive-oriented.

This scope note makes that policy explicit.

### MVP completion policy

For the bounded MVP:

- keyword/directive completion is enabled
- identifier completion is **not** enabled

This is true regardless of `flat` vs `hierarchical`.

### Why identifier completion stays off

Identifier completion depends on:

- scope/index behavior
- canonical element keys
- selector insertion behavior
- `!identifiers` policy

Until those are all stable together, identifier completion should not be exposed as a half-semantic feature.

This restriction is specifically about semantic identifier insertion.
It does **not** affect the style-property completion already available inside parsed `element_style` and `relationship_style` blocks, because those suggestions do not depend on `!identifiers`, scope resolution, or canonical element keys.

### Future identifier completion policy

When identifier completion is eventually added:

- in `flat` mode, offer simple canonical element keys and flat relationship identifiers
- in `hierarchical` mode, offer canonical hierarchical element keys plus flat relationship identifiers

But hierarchical element completion should not ship as stable until selector-aware insertion behavior is implemented and tested.

## `!identifiers` and rename

Rename is out of the bounded MVP, but this note should still define the first safe rollout boundary.

### MVP rename policy

For the bounded MVP:

- rename is disabled

### First future rename candidate set

The safest first rename rollout would require all of the following:

- the target is a bindable symbol
- the target lives in one known workspace instance, or all candidate instances produce the same edit set
- all known reference sites for that symbol are within the supported bounded reference kinds
- no deferred selector/`this`/dynamic-reference behavior needs to be rewritten

### Practical consequence

That means:

- flat-mode element identifiers are the safest future starting point
- named relationship identifiers may also become early candidates because `!identifiers` does not affect them
- hierarchical element rename should remain deferred until selector reference handling exists

Why hierarchical rename should wait:

- the canonical key changes are path-like
- selector syntax rewriting is not yet in scope
- partial rename support would be too easy to get wrong

## Model-level override example

The current fixture [`crates/structurizr-lsp/tests/fixtures/directives/identifiers-directive-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/directives/identifiers-directive-ok.dsl) is a good example of the bounded rule:

```dsl
workspace {
    !identifiers flat

    model {
        !identifiers hierarchical

        system = softwareSystem "System" {
            api = container "API" {
                worker = component "Worker"
            }
        }

        !element system.api.worker {
            ...
        }
    }
}
```

Bounded interpretation:

- effective mode for the model subtree is `hierarchical`
- canonical keys are:
  - `system`
  - `system.api`
  - `system.api.worker`
- the `!element system.api.worker` selector remains deferred
- keyword/directive completion remains unaffected
- identifier completion and rename remain deferred for this hierarchical element path

That is exactly the kind of “compute the internal model first, expose the syntax later” split this note is aiming for.

## Recommended implementation order for these rules

1. compute canonical binding keys during analysis for bindable symbols
1. keep element and relationship binding tables separate
1. apply exact-match resolution for the supported bounded reference kinds
1. gate completion and rename according to the rules above
1. only then expand into selector reference handling

This preserves a clean internal model without overextending the first external feature set.

## What this unblocks

Once these scope rules are written down:

- the analysis crate can compute stable canonical keys
- the LSP handlers can define exactly when a supported reference resolves
- the remaining deferred area becomes much narrower and clearer

Most importantly, it prevents the project from drifting into accidental pseudo-runtime behavior just to make a few navigation cases work.

## Sources

- [`docs/lsp/02-design/workspace-discovery-includes.md`](workspace-discovery-includes.md)
- [`docs/lsp/02-design/analysis-crate-skeleton.md`](analysis-crate-skeleton.md)
- [`docs/lsp/02-design/first-pass-symbol-extraction.md`](first-pass-symbol-extraction.md)
- [`docs/lsp/02-design/bounded-mvp-handlers.md`](bounded-mvp-handlers.md)
- [`docs/lsp/90-history/syntax-audit-directive-nodes.md`](../90-history/syntax-audit-directive-nodes.md)
- [`crates/structurizr-lsp/tests/fixtures/directives/identifiers-directive-ok.dsl`](../../../crates/structurizr-lsp/tests/fixtures/directives/identifiers-directive-ok.dsl)
- `https://docs.structurizr.com/dsl/identifiers`
