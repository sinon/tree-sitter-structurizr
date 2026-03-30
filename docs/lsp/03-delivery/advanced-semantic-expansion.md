# Structurizr DSL advanced semantic expansion

This note defines how the future Structurizr DSL server should grow **after** the bounded MVP is stable.

It is the Phase 6 companion to:

- `docs/lsp/02-design/bounded-mvp-handlers.md`
- `docs/lsp/02-design/scope-rules.md`
- `docs/lsp/02-design/workspace-index.md`
- `docs/lsp/01-foundations/query-ownership.md`

Its job is to keep post-MVP work deliberate instead of letting the server expand through ad hoc requests for “just one more feature”.

## Why this note exists

By the time the bounded MVP exists, the project should already have:

- syntax diagnostics
- document symbols
- keyword/directive completion
- go-to-definition for the bounded identifier set
- find-references for the same bounded set
- workspace instances and bounded workspace indexes
- Zed integration with grammar + LSP working together

At that point, the pressure usually changes.

Instead of asking “can this be an LSP at all?”, the questions become:

- can we add rename now?
- can we add hover?
- can we support `system.api`?
- can we color unresolved references semantically?
- can we surface workspace symbols?

Those are all reasonable questions.

But if they are implemented in whichever order is most tempting in the moment, the server will drift into:

- duplicated logic across handlers
- partial scope support that makes rename unsafe
- protocol polish that outruns the semantic model
- runtime-like behavior the project never intended to own

This note is the guardrail against that drift.

## Main conclusion

The server should grow in **ordered tracks**, not as one undifferentiated “advanced features” bucket.

Recommended order:

1. allow narrow syntax-backed completion refinements that rely only on stable parse context
2. complete the deferred core reference/scope model for the current bounded symbol families
3. add richer **read-only** semantic features on top of that
4. add **edit-capable** semantic features only when reference coverage is strong enough
5. add presentation/polish features only if they provide value beyond Tree-sitter-native editor behavior

The most important meta-rule is:

- do not add an advanced feature unless the analysis layer can already explain its answer with stable workspace facts

## Preconditions for entering Phase 6

Do not start this phase just because the LSP binary exists.

Start it when the bounded MVP is stable enough that:

- the core workspace index is tested against realistic multi-file fixtures
- include diagnostics work at directive sites
- open fragments behave conservatively across zero/one/multiple candidate workspace instances
- bounded definition/references behavior is predictable in Zed and in tests
- the server is still clearly editor-oriented, not runtime-oriented

If any of those are still shaky, post-MVP expansion should wait.

## Shared guardrails for every advanced feature

Every later feature in this note should obey the same rules.

### Rule 1: extend analysis facts first

If a feature needs new syntax understanding, extend:

- extraction facts
- scope rules
- workspace-index facts

before extending handlers.

Do not let handlers become the place where deferred syntax starts getting resolved ad hoc.

### Rule 2: prefer read-only features before edit features

If a semantic area is only partially understood, prefer:

- hover
- workspace symbols
- diagnostics

before:

- rename
- completion that inserts identifiers
- code actions that rewrite source

Read-only features are safer places to expose a growing semantic model.

### Rule 3: do not outrun scope rules

If a feature depends on:

- selector semantics
- `this`
- named dynamic relationship references
- multi-context fragment behavior

and those rules are not yet explicit and tested, the feature should wait.

### Rule 4: keep Zed’s Tree-sitter strengths in place

The presence of a richer LSP should **not** become an excuse to move editor-native behavior out of:

- syntax queries
- outline queries
- bracket queries
- indentation/folding queries

unless there is a concrete portability or semantic reason to do so.

### Rule 5: do not widen symbol families and edit features at the same time

If the project decides to add new declaration families such as:

- deployment surfaces
- custom elements
- archetype-driven declarations

that should happen as a separate semantic expansion slice, not in the same rollout as rename or identifier completion.

Otherwise it becomes too hard to tell whether failures come from:

- new symbol kinds
- new scope rules
- or new edit behavior

## Expansion tracks

The cleanest way to think about Phase 6 is five tracks.

## Track 0: remaining syntax-backed completion refinements

Style-property name completion inside parsed style blocks is now landed.
Before broader semantic expansion starts, the server can still add small completion improvements that rely only on stable parse context and static DSL tables.

### What belongs in this track

- finite value completion inside `element_style` blocks
- finite value completion inside `relationship_style` blocks
- optional `properties {}` block scaffolding inside style-rule blocks

### Why this track is separate

- it does not require workspace indexes or symbol resolution
- it does not depend on `!identifiers` or rename-grade scope confidence
- it is closer to the existing keyword/directive completion slice than to semantic identifier completion

### Recommended rollout inside this track

1. define activation contexts and non-semantic value tables first
2. add value suggestions only when the grammar or fixtures already make the allowed values explicit
3. leave arbitrary `properties {}` entries additive rather than trying to validate them semantically

### Important guardrail

The grammar still accepts generic identifier-based style keys, so this completion should be additive editor guidance rather than a validity gate.

## Track 1: complete the core deferred reference model

Before adding ambitious editor polish, the project should complete the most important deferred semantic surfaces for the **existing** core symbol families.

That means deferring broad new domains until the current core domains are explained well.

### What belongs in this track

- selector-based element references such as `system.api`
- `!element` / `!relationship` selector lookup targets
- `this`
- omitted-source relationship shorthand
- named dynamic relationship references
- any other direct follow-on work needed to make the current core symbol families semantically coherent

### Why this track comes first

The current planning set intentionally computes:

- canonical hierarchical element keys

before it resolves:

- selector reference syntax

That was the right bounded-MVP decision.

But it means the first post-MVP work should close that gap before pretending rename, identifier completion, or advanced code actions are safe.

### Recommended rollout inside this track

#### 1A. Selector references for current core element keys

Add extraction + resolution for:

- hierarchical element selectors like `system.api`

Use the existing canonical key model from `docs/lsp/02-design/scope-rules.md`.

Important rule:

- selector matching should stay exact and canonical
- do not add fuzzy or partial selector matching

This is the feature that unlocks:

- hierarchical identifier completion
- later hierarchical rename
- more accurate hover for hierarchical references

#### 1B. Reuse selector machinery for `!element` / `!relationship`

Do not build a one-off lookup path for:

- `!element`
- `!relationship`

Instead, once selector resolution exists, route these through the same resolution machinery where possible.

Why:

- fewer parallel lookup systems
- cleaner testing
- less drift between navigation surfaces

#### 1C. Add containing-context references like `this`

Only after selector reference handling is stable should the project add:

- `this`
- omitted-source relationship shorthand

Those cases depend on the server understanding more than just raw identifier text.

They need:

- containing declaration context
- containing symbol ancestry
- clear failure behavior when the containing context is partial or invalid

They should not be guessed.

#### 1D. Add named dynamic relationship references last in this track

Named dynamic relationship references mix:

- relationship symbol semantics
- view semantics
- deferred reference surfaces

So they should stay at the back of the current-core reference-expansion queue.

### Exit criteria for Track 1

- selector references for the current core symbol families are explained and tested
- `!element` / `!relationship` lookups are no longer special unresolved holes
- the server still returns conservative results instead of speculative guesses
- the scope rules note can be updated without contradiction rather than being bypassed

## Track 2: add richer read-only semantic features

Once the core deferred reference model is stronger, the next safest growth area is read-only semantic value.

### Features in this track

- richer hover
- workspace symbols
- broader diagnostics quality
- carefully chosen expansion of symbol families for read-only surfaces

## 2A. Richer hover

Hover should grow before rename.

Why:

- it exposes semantic understanding without rewriting source
- it helps validate whether the index model is actually useful
- it reveals whether the project is drifting into runtime output instead of source-derived editor help

### Recommended first hover content

For supported element symbols:

- symbol kind
- display name
- binding name when present
- canonical key when useful
- parent chain when useful
- source file / declaration context

For supported named relationships:

- relationship identifier
- source and destination targets when resolved
- description text when present

### What hover should avoid

Do **not** turn hover into:

- a runtime summary of the whole Structurizr model
- diagram rendering
- execution-time validation output
- a place to dump every property/value in the file

Keep it source-derived and editor-friendly.

## 2B. Workspace symbols

Workspace symbols should read from:

- workspace indexes

not from:

- one global folder scan
- ad hoc handler-side tree walking

### Recommended bounded post-MVP policy

- return symbols grouped conceptually by workspace instance
- include enough context in labels/details/container fields that users can tell which root workspace a shared fragment symbol belongs to
- do not collapse semantically distinct instance-scoped symbols just because they come from the same physical file

This is especially important for:

- shared fragments
- different workspace-level `!identifiers` modes

### New symbol families

If the project wants workspace symbols for new declaration families beyond the bounded MVP:

- add those families one at a time
- start with read-only surfaces such as document/workspace symbols and hover
- defer rename and completion for them until their scope rules are explicit

That lets the server grow breadth without taking on too much edit risk.

## 2C. Diagnostics polish

The bounded MVP already reserves space for:

- include diagnostics
- duplicate-definition diagnostics
- unresolved-reference diagnostics

The post-MVP phase should improve those **messages and classification**, not turn them into runtime validation.

### Good candidates

- clearer duplicate-binding diagnostics that distinguish element vs relationship conflicts
- clearer unresolved-reference diagnostics that distinguish “no match” from “ambiguous”
- better multi-context fragment messaging when semantic answers differ by workspace instance
- better messaging around remote includes remaining unresolved

### Severity discipline

Keep severity proportional:

- syntax and invalid local include structure -> error
- missing local include target -> error
- unresolved but bounded semantic reference -> warning or error depending on confidence and UX value
- remote include unsupported in MVP -> warning or information
- multi-context ambiguity in open fragments -> information or warning, not hard failure

### What to avoid

Do not expand diagnostics into:

- “full Structurizr correctness”
- plugin/script execution problems
- every runtime-style validation that upstream could theoretically do

## Track 3: add edit-capable semantic features

This is the most dangerous track.

It should begin only after the project has enough reference coverage to produce safe edits.

### Features in this track

- identifier completion
- rename

## 3A. Identifier completion

Identifier completion should remain behind rename-grade scope confidence.

### Recommended rollout

#### First

- flat-mode element identifiers
- flat relationship identifiers where the syntax site already expects them

#### Later

- hierarchical element identifiers only after selector insertion behavior is explicit and tested

This matches the scope rules note:

- hierarchical identifiers should not ship as stable completion behavior until selector-aware insertion exists

### Important rule

Completion should insert:

- canonical keys for the active scope model

It should not insert:

- display names
- guessed shorthand
- path fragments that the current resolver does not actually support

## 3B. Rename

Rename should be the last major semantic feature of the core post-MVP phase, not the first.

### Recommended rollout order

#### Step 1: flat-mode element rename

Only when:

- the target is a bindable core element symbol
- the target has one known workspace instance, or all candidate instances produce the same edit set
- all known reference sites for that target are in supported reference kinds

This is the safest first rename slice and already follows the logic in `docs/lsp/02-design/scope-rules.md`.

#### Step 2: named relationship rename

This can come soon after flat element rename because:

- relationship identifiers are unaffected by `!identifiers`
- the target family is smaller and more explicit

But it still requires:

- complete supported reference coverage for the relationship surfaces being renamed

#### Step 3: hierarchical element rename

Only after:

- selector references are fully supported for the intended syntax surfaces
- selector rewrite behavior is explicit and tested
- there are no deferred reference sites that would silently miss updates

This should stay deferred until the project can rewrite path-like identifiers confidently.

### Hard stop rules for rename

Rename should still return “not supported” / no result when:

- selector semantics are still deferred for that target family
- `this` or dynamic reference sites would be missed
- multiple workspace instances produce different edit sets
- a duplicate binding set makes the target non-unique

Better no rename than a wrong rename.

## Track 4: presentation and polish features

These features are worthwhile only if they solve a real editor problem after the semantic model is already solid.

### Features in this track

- semantic tokens
- code actions
- query layering revisit

## 4A. Semantic tokens

Semantic tokens should be treated as optional polish, not as proof that the server is mature.

### When they are justified

Only add semantic tokens when they can express distinctions Tree-sitter queries cannot already cover well, for example:

- definition vs reference
- resolved vs unresolved reference
- named relationship identifiers as a distinct semantic concept
- later symbol-family distinctions that matter to editor UX

### Zed-specific rule

For Zed, semantic tokens should be planned around:

- `"combined"` mode first

not:

- replacing Tree-sitter highlighting entirely

Zed already has strong Tree-sitter-native highlighting, so semantic tokens should complement it, not try to re-implement it all.

### Token-type rule

Prefer:

- standard LSP token types/modifiers first

Only introduce:

- custom token types

when the UX value is clear enough that the extension can justify shipping matching `semantic_token_rules.json` defaults later.

### What to avoid

Do not spend early post-MVP energy on semantic tokens if the same effort would be better spent on:

- selector resolution
- rename safety
- better diagnostics

Those features generally produce more practical editor value first.

## 4B. Code actions

Code actions should have the highest bar of any later feature in this note.

### First principle

Do not add a code action unless:

- the triggering diagnostic or semantic state is stable
- the fix is deterministic
- the edit is explainable and testable
- the server is not guessing user intent

### Candidate shape for a first code-action slice

If code actions are added at all, they should start with:

- local, text-rewrite-only fixes for diagnostics with one obvious answer

They should **not** start with:

- broad refactors
- project-wide speculative rewrites
- file-creation workflows
- runtime-proxy fixes

It is acceptable if the first post-MVP server still has:

- no code actions

until a truly safe and useful diagnostic/fix pair emerges.

## 4C. Query layering revisit

Once the advanced semantic model exists, the project should revisit whether more portable query surfaces are justified.

### Still keep extension-owned

- `outline.scm`
- `brackets.scm`
- `textobjects.scm`

These remain Zed-owned unless another real consumer creates pressure to move them.

### Grammar-repo candidates

The strongest portable candidates remain:

- `tags.scm`
- later, possibly `locals.scm` if multiple consumers benefit from it

### Important rule

Do not move a query into the grammar repo just because:

- the LSP exists

Move it only if:

- it is genuinely portable
- more than one consumer benefits
- it reduces duplicated logic instead of creating two competing sources of truth

## Feature gates table

The safest way to keep Phase 6 honest is to write down what each feature depends on.

| Feature | Must already exist | First safe slice | Keep deferred until |
| --- | --- | --- | --- |
| Selector-based references | canonical key model, workspace index, explicit selector extraction | exact canonical selectors for current core element symbols | fuzzy or partial selector matching |
| `!element` / `!relationship` lookup resolution | selector resolver | reuse same selector-resolution path | separate one-off resolver logic |
| `this` / omitted-source relationships | containing-symbol context rules | current core element families in well-defined contexts | malformed or ambiguous containing contexts |
| Named dynamic relationship references | relationship symbol model, view semantics, selector/reference stability | bounded dynamic-reference surfaces | broad dynamic-view heuristics |
| Richer hover | stable symbol/reference resolution | core elements + named relationships | runtime-style model rendering |
| Workspace symbols | workspace index, reverse document-to-instance membership | instance-scoped symbols with root context | folder-global symbol bags |
| Identifier completion | stable canonical keys + supported insertion behavior | flat-mode element + relationship identifiers | hierarchical insertion before selector support |
| Rename | supported reference coverage + unique binding resolution | flat-mode core element rename | hierarchical rename before selector rewrite coverage |
| Semantic tokens | stable semantic categories | combined-mode semantic distinctions that Tree-sitter cannot express well | full replacement of Tree-sitter highlighting |
| Code actions | stable diagnostics with deterministic edits | local, text-only fixes with one obvious answer | speculative project-wide rewrites |

## Testing expectations for advanced features

Every Phase 6 addition should come with tests at the right layer.

## Analysis-layer tests

Use these for:

- new `ReferenceKind`s
- selector-resolution logic
- expanded scope rules
- duplicate/ambiguous classification

## Workspace tests

Use these for:

- shared fragments
- different identifier modes across roots
- workspace-symbol aggregation
- rename safety across multi-file instances

## LSP-level tests

Use these for:

- hover payloads
- workspace symbol queries
- rename edit sets
- semantic token payloads
- code action results

## Manual/editor checks

Use Zed for:

- hover presentation sanity
- workspace symbol usability
- semantic token layering in `"combined"` mode
- extension behavior when the server is still downloaded or locally overridden

## Recommended rollout order across the whole phase

If the project wants one practical order rather than a menu, use this:

1. selector references for current core element symbols
2. `!element` / `!relationship` lookup support
3. `this` and omitted-source relationships
4. richer hover
5. workspace symbols
6. diagnostics polish
7. flat-mode identifier completion
8. flat-mode element rename
9. named relationship rename
10. hierarchical completion/rename only after selector rewrite support
11. semantic tokens if a real UX gap remains
12. code actions only when a deterministic first class emerges

This order optimizes for:

- semantic coherence first
- read-only value before source editing
- protocol polish last

## What this note intentionally does not settle

This note does **not** force:

- one exact future symbol-family expansion list
- one exact hover markdown format
- one exact token legend
- one exact code-action catalog

Those can still evolve.

What this note **does** settle is the direction:

- finish core semantic gaps first
- then expose read-only value
- then add source-editing features
- then add polish only where it still earns its keep

## What this unblocks

Once this note exists:

- the last open roadmap phase has a concrete implementation direction
- future sessions can add advanced features without re-deciding the whole ordering each time
- rename, hover, semantic tokens, and code actions now have explicit entry gates instead of vague “someday” status
- the server can keep growing as an editor tool without quietly becoming a Structurizr runtime proxy

That is the final planning boundary the current research set was missing.

## Sources

- `docs/lsp/03-delivery/roadmap.md`
- `docs/lsp/01-foundations/capability-matrix.md`
- `docs/lsp/02-design/bounded-mvp-handlers.md`
- `docs/lsp/02-design/scope-rules.md`
- `docs/lsp/02-design/workspace-index.md`
- `docs/lsp/01-foundations/query-ownership.md`
- `docs/lsp/03-delivery/zed-extension-language-server-wiring.md`
- `https://zed.dev/docs/extensions/languages`
