# Structurizr DSL workspace discovery and include resolution

> Status: implemented in bounded form.
>
> Workspace discovery and include resolution now exist in-repo. This note
> remains the design contract for that behavior and the place to document later
> expansion without drifting into runtime semantics.

This note defines how the future analysis crate and LSP should discover `.dsl` files and handle `!include` without drifting into full runtime behavior.

It is meant to make Phase 3 of `docs/lsp/03-delivery/roadmap.md` concrete enough that future implementation work can start from a stable model instead of re-deciding filesystem and include semantics ad hoc.

## Why this needs its own note

Workspace discovery and include resolution are where the future server stops being “just a parser wrapper”.

This is the first place where we need explicit policy for:

- what counts as a workspace input
- how `!include` turns one file into a multi-file workspace
- how to keep editor behavior deterministic without pretending to execute the full Structurizr runtime

It is also the place where a naive “scan all `.dsl` files and merge them together” approach becomes dangerous.

That would be wrong because included fragments can be reused by multiple root workspaces, and those roots should not silently share one global symbol table.

## Bounded goal

The bounded MVP for workspace discovery and includes should do only this:

1. find candidate Structurizr files in the editor workspace
2. parse those files and extract `!include` facts
3. resolve explicit local include targets
4. record remote include targets without fetching them
5. detect missing local targets and simple include cycles
6. build per-workspace-instance facts that later definition/reference logic can consume

It should **not** try to do all of the following yet:

- fetch remote include URLs
- execute scripts or plugins
- fully reproduce upstream workspace expansion semantics
- settle all scope rules for nested identifiers
- treat every file under a workspace folder as belonging to one semantic workspace

## Reference-backed constraints

The current DSL reference says:

- `!include <file|directory|url>`
- included content is simply inlined into the parent document
- local file and directory targets are specified by relative paths
- local targets are expected to live in the same directory as the parent file or a subdirectory of it
- URL targets point to a single HTTPS DSL file

That gives the future server a few solid guardrails:

- include resolution is relative to the including document
- local includes should not be treated as arbitrary filesystem imports
- remote URLs are part of the language surface, even if the LSP chooses not to fetch them initially

## Recommended ownership split

The future analysis crate should own:

- workspace scanning
- include-fact extraction
- include-target normalization
- include-graph construction
- include diagnostics as stable analysis facts

The future LSP crate should own:

- receiving workspace folders and open-document events
- scheduling rescans/rebuilds
- mapping include diagnostics into LSP diagnostics
- deciding when to recompute workspace instances after file changes

This keeps filesystem and include policy transport-agnostic.

## Core model

The design gets simpler if we distinguish **documents** from **workspace instances**.

## Documents

A document is one physical file identified by URI/path.

A document may be discovered from:

- workspace scanning
- an open editor buffer
- explicit inclusion from another document

Useful future metadata:

- `DocumentId`
- `DocumentSnapshot`
- `DiscoverySource` (`WorkspaceScan`, `OpenDocument`, `ExplicitInclude`)
- extracted `IncludeDirective` facts

## Workspace instances

A workspace instance is a semantic expansion rooted at one entry document.

That entry document is usually a file containing a `workspace` block.

This distinction matters because the same fragment file may be included by multiple entry documents.

If two root workspaces both include `model/people.dsl`, that does **not** mean they now share one semantic workspace.

Instead:

- one physical file can participate in multiple workspace instances
- each workspace instance gets its own include closure
- semantic features should resolve against a chosen workspace instance, not a single folder-wide symbol bag

## Entry documents and fragments

The first implementation should classify parsed files into two broad roles.

## Entry document

A document that can act as the root of a workspace instance.

For the bounded MVP, that should mean:

- a file that contains a `workspace` declaration

Later we may expand this if upstream fragment usage makes a broader rule useful, but that is the safest first boundary.

## Fragment document

A document that is parseable and may be included, but does not stand on its own as a root workspace instance.

Examples:

- a file containing only `model { ... }`
- a file containing only `views { ... }`
- a file containing only directives plus a fragment block

## Discovery pipeline

The future server should use a two-lane discovery model.

## Lane 1: general workspace scan

Use the `ignore` crate to find candidate `.dsl` files under the editor's workspace folders.

This lane should:

- respect normal ignore rules for broad scanning
- discover unopened root files for workspace symbols later
- avoid custom filesystem walking where `ignore` already does the job

This lane is intentionally conservative.

It is for “find likely Structurizr files in the project”, not “resolve every dependency edge”.

## Lane 2: explicit include resolution

Explicit `!include` targets should be resolved even if they would not have been found by the general scan.

This is already called out in `docs/lsp/03-delivery/roadmap.md`, and it is the right rule.

Implications:

- an ignored file can still matter if some parsed document explicitly includes it
- an explicitly included directory can still matter even if the directory would otherwise be skipped by the general scan
- include resolution is a dependency walk, not just a filtered workspace listing

Ignore rules should therefore apply to **background discovery**, not to **explicit dependency edges**.

## Local include resolution rules

For the bounded MVP, local include resolution should follow these rules.

### Rule 1: resolve relative to the parent document directory

The raw include value should be interpreted relative to the including document's directory.

That applies to:

- local files
- local directories

### Rule 2: record raw syntax first, resolve second

The syntax layer should extract the raw include value and its range.

The resolver layer should then decide whether that raw value represents:

- a local file
- a local directory
- a remote HTTPS URL
- an invalid/unsupported target

This keeps the parse layer simple and matches the directive audit.

### Rule 3: explicit includes bypass ignore filters, not safety checks

An explicit include may reach a file that the general scan skipped.

But it should **not** bypass path-safety checks.

For the bounded MVP, local includes should be rejected or diagnosed when normalization shows that they escape the including document's allowed subtree.

That means we should treat these as invalid:

- `!include ../shared.dsl`
- any symlink/canonical-path outcome that escapes the including file's directory tree

This matches the public docs better than treating `!include` as an unrestricted filesystem import.

### Rule 4: do not require general-scan file extensions for explicit local files

The general scan should only look for `.dsl` files.

But an explicit local file include should be resolved by path rather than by extension filtering.

Why:

- the language reference defines `!include <file|directory|url>`, not `!include <*.dsl>`
- users may keep fragments with unconventional filenames

If an explicitly included file is not valid DSL, that should surface as a parse problem, not as “file not discovered”.

### Rule 5: directory includes should expand deterministically

The public docs say that included content is inlined in the order files are discovered.

For editor tooling, deterministic behavior matters more than whatever order the host filesystem happens to return.

So the future implementation should make directory expansion stable by sorting the discovered child paths before processing them.

If later upstream-audit work shows that a more specific traversal rule is needed, we can adjust the exact expansion rule without rewriting the whole model.

### Rule 6: remote includes are recorded, not fetched, in MVP

The MVP should recognize HTTPS include URLs as first-class include facts, but it should not fetch them.

Instead:

- record that the include target is remote
- exclude it from local workspace expansion
- optionally surface an informational or warning diagnostic explaining that remote includes are not resolved yet

This keeps the first server:

- deterministic
- offline-friendly
- free from third-party fetch behavior during editing

Remote fetching can be revisited later as an explicit opt-in feature if it proves worthwhile.

## Graph construction

The future analysis crate should build an include graph from parsed include facts.

Useful future concepts:

- `IncludeTarget`
- `ResolvedInclude`
- `IncludeResolutionStatus`
- `WorkspaceInstance`

Each resolved local include edge should retain:

- parent document ID
- child document ID
- directive source range in the parent
- target kind (`file`, `directory`, `remote`, `invalid`)
- resolution status

This is important because diagnostics should point back to the directive site in the parent document, not only to the child file.

## Cycle handling

Cycle handling should stay file-level and simple in the bounded MVP.

We only need enough behavior to avoid infinite expansion and produce helpful diagnostics.

Recommended policy:

- normalize document paths/URIs before cycle checks
- detect cycles on the current expansion stack
- stop traversing once a cycle edge is found
- emit diagnostics on the directive site(s) participating in the cycle

We do **not** need to fully model every semantic consequence of cyclical inlining before shipping useful diagnostics.

## Workspace instance selection

This is the most important design choice in this note.

The server should **not** use one giant symbol table per workspace folder.

Instead, it should build workspace instances rooted at entry documents.

### Why this matters

A folder can contain:

- multiple independent Structurizr workspaces
- fragments that are never included
- one fragment reused by multiple workspaces

If we merge everything together, definition/reference results will become wrong very quickly.

### Recommended rule

Semantic features should run against a chosen workspace instance:

- root document = the active workspace instance root
- instance contents = that root plus its transitive local include closure

### What to do for open fragments

An included fragment can be in one of three states:

1. included by no known workspace instance
2. included by exactly one workspace instance
3. included by multiple workspace instances

Recommended bounded-MVP behavior:

- **zero contexts**: syntax-only behavior plus local parse facts
- **one context**: full bounded semantic behavior using that workspace instance
- **multiple contexts**: keep syntax diagnostics, but avoid context-sensitive semantic claims unless the answer is identical across all candidate contexts

That is more conservative than pretending every fragment has one obvious semantic owner.

## Diagnostics to support first

The first include-related diagnostics should stay narrow.

Recommended initial set:

- missing local include target
- local include escapes allowed subtree
- explicit include cycle
- unsupported remote include resolution in MVP

These should be separate from:

- parse errors inside included files
- unresolved identifier diagnostics
- duplicate-definition diagnostics

Those later diagnostics may depend on workspace-instance context and should not be conflated with include resolution itself.

## Interaction with file watching and invalidation

The future LSP should invalidate workspace-instance facts when:

- an open document changes
- a discovered `.dsl` file is created, renamed, or deleted
- a file or directory targeted by an explicit include changes

The important part is not the exact watch mechanism yet.

The important part is that invalidation should follow dependency edges:

- changing a fragment should invalidate every workspace instance that includes it
- changing a directory include's contents should invalidate parents that expand that directory

This is another reason the include graph belongs in the analysis layer rather than being recomputed ad hoc in each LSP handler.

## Testing shape to aim for

The current fixture slice under `crates/structurizr-grammar/tests/fixtures/lsp/` is enough for syntax-oriented include coverage, but not yet for analysis/workspace behavior.

When the analysis crate starts, add workspace-level tests that cover at least:

- one root workspace including local file fragments
- one root workspace including a directory
- missing local include target
- include cycle
- ignored-but-explicitly-included file
- one fragment included by multiple roots
- remote include recorded but not fetched

Suggested future area:

```text
tests/lsp/workspaces/
  includes/
  cycles/
  missing/
  shared-fragments/
  ignored-explicit/
```

The exact filenames can change, but the scenarios matter.

## Clear non-goals for this slice

This note intentionally does **not** settle:

- `workspace extends <file|url>`
- variable or constant substitution inside include values
- full directory traversal semantics beyond “be deterministic”
- how include order should affect later duplicate-definition policy
- execution-time behavior from scripts/plugins/components

Those are adjacent concerns, but mixing them into the first workspace-discovery implementation would blur the bounded-MVP line too early.

## Recommended next implementation sequence

1. Create the analysis crate skeleton with stable document/include fact types.
2. Implement parse + include-fact extraction for single documents.
3. Add workspace scanning with `ignore`.
4. Add explicit local include resolution and cycle detection.
5. Build workspace instances rooted at entry documents.
6. Only then layer definition/reference logic on top of those workspace-instance facts.

That keeps workspace discovery and include handling foundational instead of bolted on after symbol logic already exists.

## Sources

- `docs/lsp/03-delivery/roadmap.md`
- `docs/lsp/01-foundations/overview.md`
- `docs/lsp/01-foundations/capability-matrix.md`
- `docs/lsp/90-history/syntax-audit-directive-nodes.md`
- `crates/structurizr-grammar/tests/fixtures/lsp/includes/`
- `https://raw.githubusercontent.com/structurizr/structurizr.github.io/main/dsl/71-language.md`
- `https://docs.structurizr.com/dsl/includes`
