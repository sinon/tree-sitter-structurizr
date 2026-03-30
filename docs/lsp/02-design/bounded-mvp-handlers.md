# Structurizr DSL bounded MVP handlers

This note defines the first user-visible LSP handler slice for the future Structurizr server.

It sits on top of:

- `docs/lsp/02-design/analysis-crate-skeleton.md`
- `docs/lsp/02-design/first-pass-symbol-extraction.md`
- `docs/lsp/02-design/lsp-crate-skeleton.md`

The goal is to make the first handler implementation concrete enough that future work can implement the server feature-by-feature without re-deciding scope for every request type.

## Why this note exists

We already have:

- a bounded MVP feature list
- a planned analysis crate
- a planned LSP crate
- a first-pass extraction contract

What we still needed was a concrete answer to:

- what each first handler should consume
- what it should return
- what it should do when the request lands in explicitly deferred scope
- which parts should stay editor-native instead of being forced through LSP

This note is that contract.

## Main conclusion

The bounded MVP handler slice should implement exactly these user-visible behaviors first:

1. syntax diagnostics
2. document symbols
3. keyword/directive completion
4. go-to-definition for the bounded identifier set
5. find-references for the same bounded identifier set

And it should do so by consuming analysis facts rather than re-walking syntax trees in handlers.

That means:

- diagnostics come from `DocumentSnapshot` and later workspace facts
- document symbols come from structural `Symbol` facts
- completion uses syntax/context plus directive knowledge, not full semantic completion
- definition and references use only bindable symbols and supported `ReferenceKind`s

Everything else should remain explicitly out of scope rather than being partially guessed.

## Handler set for the bounded MVP

The LSP crate skeleton already names the relevant handler modules.

For the first useful server, these should be the active MVP handler surfaces:

- `textDocument/publishDiagnostics`
- `textDocument/documentSymbol`
- `textDocument/completion`
- `textDocument/definition`
- `textDocument/references`

Lifecycle and text-sync handlers are required plumbing, but they are not the feature slice this note is focused on.

## Shared behavioral rules

All bounded-MVP handlers should follow these rules.

### Rule 1: consume analysis facts, not raw parse trees

Handlers should read:

- `DocumentSnapshot`
- extracted `Symbol` and `Reference` facts
- directive facts such as `IncludeDirective` and `IdentifierModeFact`
- later workspace/index facts where the feature truly needs them

If a handler needs a syntax detail that is not exposed yet, extend the analysis layer rather than bypassing it.

### Rule 2: return conservative results when scope is deferred

For deferred syntax such as:

- `this`
- `a.b.c`
- dynamic relationship references
- filtered-view key/tag references

the bounded-MVP handlers should:

- return no result or an empty collection
- avoid fabricating a best guess
- stay consistent across requests

“No confident answer” is better than a wrong answer for navigation features.

### Rule 3: prefer empty results over error notifications for unsupported semantic scope

If the user requests definition or references on syntax that is intentionally out of scope, the server should generally return:

- `null` for singular navigation responses
- `[]` for collection responses

This keeps the server calm and editor-friendly.

Diagnostics are for actual problems in the source, not for every unsupported editor query.

### Rule 4: do not duplicate editor-native syntax behavior unnecessarily

Zed already has:

- Tree-sitter grammar loading
- syntax highlighting
- folding/indentation queries
- extension-owned outline queries

The LSP should therefore focus on semantic value first.

It is fine if document symbols overlap somewhat with editor outline features, but the MVP should not try to replace editor-native query behavior wholesale.

Plain folder/file arguments should also stay out of bounded semantic navigation.
Specifically:

- `!docs <path> [fully qualified class name]`
- `!adrs <path> [adrtools|madr|log4brains|fully qualified class name]`
- file-valued top-level `!include <path>`

These are editor-local path links, not semantic symbol references.
The bounded MVP should therefore keep them out of semantic reference resolution and out of `textDocument/references`.
When the server exposes these spans as protocol links, it should use `textDocument/documentLink` as the primary surface.
Because Zed does not yet surface `textDocument/documentLink`, the current implementation also answers `textDocument/definition` on these spans as a compatibility fallback for Cmd-click navigation.
For directory-valued targets such as `!docs` and `!adrs`, that fallback resolves to concrete files inside the directory when any exist because Zed expects file locations rather than directory URIs for definition targets.

## 1. Diagnostics handler

This is the first required user-visible handler because it establishes the trust model for the rest of the server.

### Inputs

Primary inputs:

- syntax diagnostics from `DocumentSnapshot`
- later, workspace/include facts for include errors
- later, resolved symbol/index facts for bounded semantic diagnostics

### Output

Publish diagnostics to the client for the current document.

### MVP behavior

In the first bounded handler slice, diagnostics should be layered.

#### Layer A: always publish syntax diagnostics

On open/change:

- convert parse diagnostics from the latest snapshot
- publish them immediately

This layer is unconditional.

#### Layer B: do not require semantic diagnostics yet

The handler design should reserve space for later semantic diagnostics, but the initial bounded handler slice does **not** need to implement all of them immediately.

When those later arrive, they should be added in this order:

1. missing/cyclic include diagnostics
2. unresolved-reference diagnostics for the bounded identifier set
3. duplicate-definition diagnostics for the bounded identifier set

### Deferred behavior

Do not try to publish semantic diagnostics for:

- `this`
- hierarchical selectors
- dynamic relationship references
- broader runtime validation

### Important rule

Include diagnostics should be attached to the directive site in the including document, not only to the missing or cyclic target.

That matches the workspace/include design note and will feel much more useful in editors.

## 2. Document symbols handler

This is the easiest bounded-MVP navigation surface after syntax diagnostics.

### Inputs

Use structural `Symbol` facts from the latest document snapshot.

### Output

Return `DocumentSymbol` results with hierarchical nesting where possible.

### MVP behavior

Document symbols should include:

- `person`
- `software_system`
- `container`
- `component`
- optionally named `relationship` symbols if the UI value proves worthwhile

The most important thing is that the core declaration tree is present and source-ordered.

### Naming policy

Use the `display_name` from extracted `Symbol` facts as the main label.

If later the UI wants richer labels such as:

- `User (user)`
- `System (system)`
- `Uses (rel)`

that should be treated as display policy layered above the extracted facts.

The first implementation can stay simple.

### Parent/child policy

Use the extracted `parent_symbol` relationships to produce nested document symbols:

- `software_system` contains `container`
- `container` contains `component`

If grouped declarations are reached through transparent `group_block` traversal, preserve the declaration nesting of the supported core declarations rather than inventing a `group` symbol level before group semantics are ready.

### Relationship to editor-native outline

Zed already has an `outline.scm`.

That means:

- document symbols in the LSP are still worth implementing for other editors and protocol-level feature parity
- but they do not need to become a richer replacement for every editor-specific outline behavior on day one

### Deferred behavior

Do not broaden document symbols yet into:

- deployment surfaces
- custom elements
- archetype instances
- view keys/tags/filtered-view references

Those can come later if the bounded symbol model expands intentionally.

## 3. Completion handler

The bounded MVP should keep completion intentionally narrow.

### Inputs

Use:

- cursor context from the latest document text/snapshot
- directive facts such as `IdentifierModeFact`
- a small fixed vocabulary of keywords/directives

Do **not** require a full semantic workspace index for the first completion slice.

### Output

Return `CompletionItem` values for:

- core keywords
- directives
- obvious statement keywords based on local syntactic context

### MVP behavior

The first completion slice should support:

- top-level workspace keywords where appropriate
- `model`, `views`, `configuration`
- core declaration keywords such as `person`, `softwareSystem`, `container`, `component`
- directive keywords such as `!include`, `!identifiers`, `!docs`, `!adrs`
- common view statement keywords such as `include`, `exclude`, `autoLayout`, `title`, `description`

This is enough to make the server feel helpful without requiring deep semantic scope resolution.

The current completion slice also supports style-property names inside `element_style` and `relationship_style` blocks.
That refinement stays syntax-backed and context-aware: it is driven by parsed style-block context and block-specific property tables, not by semantic identifier resolution.
Because the grammar still allows generic identifier-based style keys, it should stay additive rather than becoming a validity gate.

### What not to do yet

Do **not** implement first-pass identifier completion yet.

Reason:

- the capability matrix already classifies identifier completion as a semantic P1 feature
- it depends on scope/index behavior and `!identifiers` policy that is not fully settled

### `!identifiers` interaction

The presence of `IdentifierModeFact` should be recorded now, but the initial completion handler should use it conservatively.

That means:

- do not let `!identifiers` force identifier completion behavior before the scope-rules note exists
- follow the later policy captured in `docs/lsp/02-design/scope-rules.md` once identifier completion is intentionally introduced
- keep the first completion slice keyword/directive-oriented

Later work can expand completion deliberately once the semantic rules are documented.

### Deferred behavior

Do not try to provide:

- selector completion
- rename-aware completion
- relationship-reference completion in dynamic views
- full context-sensitive value completion for every DSL statement

Future style-completion work should stay focused on safe value suggestions and similar syntax-backed refinements rather than expanding into semantic identifier completion by accident.

## 4. Go-to-definition handler

This is the first handler that genuinely depends on the semantic extraction contract.

### Inputs

Use:

- bindable `Symbol` facts
- supported `Reference` facts
- later workspace/index facts when the target is cross-file

### Output

Return a single `Location`/definition result when the identifier is in the bounded supported set.

### Supported definition targets

Only these symbol classes should be definition targets in the bounded MVP:

- assigned core declaration identifiers
  - `user = person`
  - `system = softwareSystem`
  - `api = container`
  - `worker = component`
- named relationship identifiers
  - `rel = user -> system`

Declarations without a binding identifier remain structural symbols, not definition targets.

### Supported reference sites

Definition should work from these reference kinds only:

- `RelationshipSource`
- `RelationshipDestination`
- `ViewScope`
- `ViewInclude`
- `ViewAnimation`

And only when the raw identifier text can be matched against a supported bindable symbol in the current bounded context.

Here `ViewInclude` means identifier-valued `include` statements inside supported view bodies.
Here `ViewAnimation` means identifier-valued `animation` steps inside supported static and deployment view bodies.
File-valued `!include` directives and path arguments to `!docs` / `!adrs` are handled separately as syntax-backed path navigation rather than as semantic symbol resolution.

### Matching policy

The first definition implementation should stay deterministic and conservative:

- same-document facts can be used as a cheap fast path before full workspace indexing exists
- cross-file only when later workspace/index facts exist and the match is unambiguous within the bounded workspace instance
- once a workspace index exists, instance-wide uniqueness should still win over a same-document guess
- no best-guess matching across deferred scope shapes

### Expected examples

From the current fixtures, the handler should eventually support:

- `user` / `system` / `api` / `worker` inside plain relationship endpoints
- `system` / `api` in supported view `scope` fields
- `user` / `api` / `worker` / `rel` inside supported `include_statement` identifier positions
- `user` / `api` / `worker` and deployment instance identifiers inside supported `animation` blocks

### Deferred behavior

Return no definition result for:

- `this`
- omitted-source relationships
- `dynamic_relationship_reference`
- `!element system.api.worker`
- other selector-based lookups

Those cases should stay aligned with the extraction contract rather than being handled ad hoc in the LSP.
The current implementation exposes them through `textDocument/documentLink` and also through a narrow `textDocument/definition` fallback for editors such as Zed that do not surface document links yet.
If one source span resolves to multiple possible include targets across workspace contexts, the server suppresses `documentLink` for that span rather than emitting overlapping links and relies on `definition` to surface the multiple file results instead.

## 5. Find-references handler

This handler should use the same bounded semantic surface as go-to-definition.

### Inputs

Use:

- a bindable symbol target
- supported `Reference` facts
- later workspace-instance/index facts when references can live outside the current document

### Output

Return all supported reference locations for the selected bindable symbol.

### Supported symbol targets

Only:

- assigned core declaration identifiers
- named relationship identifiers

### Supported reference kinds

Only:

- `RelationshipSource`
- `RelationshipDestination`
- `ViewScope`
- `ViewInclude`
- `ViewAnimation`

As with definition, this surface is about semantic identifier references.
Plain folder/file path arguments for `!docs`, `!adrs`, and file-valued top-level `!include` stay downstream rather than becoming protocol references.

### Important consistency rule

The bounded first implementation of references should mirror definition support as closely as possible.

If a reference kind is not definition-capable yet, it should probably not appear in references either.

This avoids the confusing situation where “find references” seems to understand syntax that “go to definition” rejects.

### Relationship symbol references

For named relationships, the first handler should support references from:

- `include rel` inside supported view include statements

It should **not** yet support:

- dynamic relationship reference sites

That is explicitly deferred.

## Shared fallback behavior by request type

To keep the UX predictable, the first handler slice should follow these fallback rules.

### Diagnostics

- publish syntax diagnostics whenever available
- omit unsupported semantic diagnostics rather than guessing

### Document symbols

- return whatever structural symbols were confidently extracted
- do not fail the whole response because one part of the file is in deferred scope

### Completion

- return keyword/directive completions when context is recognized
- otherwise return an empty completion list

### Definition

- return `null` when the cursor is on unsupported/deferred syntax

### References

- return `[]` when the symbol or reference site is unsupported/deferred

## Recommended test shape for this handler slice

The LSP crate skeleton already proposes handler-focused tests.

For the bounded MVP handlers, add tests in roughly three categories.

### 1. Diagnostics tests

Verify:

- syntax diagnostics are published from parse failures
- diagnostics remain available even when semantic features are not

### 2. Symbol and navigation tests

Using the current LSP fixtures, verify:

- document symbols reflect the structural declaration tree
- definition works for supported assigned identifiers
- references work for the same bounded identifier set
- named relationships resolve from supported view include references

### 3. Completion tests

Verify:

- core keyword/directive suggestions appear in obvious contexts
- deferred semantic identifier completion is not accidentally exposed as if it were supported

### Result style

Prefer tests that assert:

- typed handler outputs where practical
- JSON payload shape only where transport details matter

That will keep failures easier to understand.

## Recommended implementation order inside the handler slice

1. wire diagnostics publication from existing snapshots
2. implement document symbols from structural `Symbol` facts
3. add keyword/directive completion
4. implement definition using bindable symbols plus supported reference kinds
5. implement references using the same bounded set

This order matches the usefulness and complexity curve already implied by the planning docs.

## What this slice should not do

The bounded handler slice should not:

- require identifier completion before scope rules are documented
- implement rename
- implement hover
- implement semantic tokens
- implement runtime-style validation
- invent ad hoc support for deferred syntax in one handler but not the others

If a request pushes on those boundaries, the right response is to extend the design docs and analysis facts first.

## What this unblocks

Once this handler contract exists, the future implementation path becomes much clearer:

- the LSP crate can wire handlers against known analysis facts
- the analysis crate knows which outputs are actually needed first
- the next scope-rules note can focus on the remaining semantic ambiguity instead of restating the whole handler surface

## Sources

- `docs/lsp/02-design/lsp-crate-skeleton.md`
- `docs/lsp/02-design/first-pass-symbol-extraction.md`
- `docs/lsp/01-foundations/capability-matrix.md`
- `docs/lsp/01-foundations/overview.md`
- `docs/lsp/01-foundations/repository-topology.md`
- `/Users/rob/dev/zed-structurizr/languages/structurizr/outline.scm`
- `tests/fixtures/lsp/`
