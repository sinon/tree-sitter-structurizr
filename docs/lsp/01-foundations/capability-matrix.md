# Structurizr DSL LSP capability matrix

This matrix is meant to keep future implementation work honest about what can be powered directly by Tree-sitter and what needs a real semantic layer.

## Key

- **Syntax-backed**: can be built mostly from the parse tree
- **Query-backed**: best driven by Tree-sitter queries
- **Semantic**: requires symbol/index resolution beyond syntax

## Feature matrix

| Feature | Primary source | Can leverage current repo? | Additional work needed | Suggested priority |
| --- | --- | --- | --- | --- |
| Syntax diagnostics | Syntax-backed | Yes | Map parse errors to LSP diagnostics with ranges/messages | P0 |
| Document symbols / outline | Query-backed or syntax-backed | Partially | Add `tags.scm` or equivalent node walker for symbol extraction | P0 |
| Selection ranges | Syntax-backed | Yes | Walk parent nodes to provide useful ancestor ranges; keep MVP tree-structured rather than scope-aware | P1 |
| Folding ranges | Query-backed | Partially | Likely unnecessary for Zed MVP because editor queries already cover this | P3 |
| Hover | Syntax + semantic | Partially | Attach resolved symbol metadata and declaration context; avoid runtime-style model rendering or validation | P2 |
| Keyword/directive completion | Syntax-backed | Yes | Add cursor-context heuristics and completion item text | P0 |
| Identifier completion | Semantic | Partially | Build in-scope symbol table and include-aware workspace index | P1 |
| Go to definition | Semantic | Partially | Start with top-level assigned identifiers and direct model element references; defer `this`, selectors, and dynamic-view relationship refs | P0 |
| Find references | Semantic | No | Start with the same bounded identifier set as go-to-definition before expanding to harder scoped cases | P0 |
| Rename | Semantic | No | Build safe edit sets with scope-aware resolution and conflict checks | P2 |
| Workspace symbols | Semantic | No | Aggregate a workspace-wide symbol index | P2 |
| Duplicate-definition diagnostics | Semantic | No | Detect duplicate identifiers in valid scopes | P1 |
| Unresolved-reference diagnostics | Semantic | No | Resolve identifiers and flag missing targets | P1 |
| Include diagnostics | Semantic | Partially | Resolve `!include` paths, detect missing files/cycles, and index included docs; start with file-resolution concerns before deeper semantics | P1 |
| View-target diagnostics | Semantic | No | Validate references from views to model elements | P2 |
| Semantic tokens | Semantic | No | Add semantic classification layer and token legend | P3 |
| Code actions | Semantic | No | Add fix generation once diagnostics stabilize | P3 |

## What the existing queries should continue to own

For Zed specifically, keep these editor-native where possible:

- highlighting
- folding
- indentation
- bracket behavior
- outline queries if Zed can use them directly

The LSP should not duplicate query-driven editor behavior just because the protocol can represent it.

## Query work worth planning early

Even if the first LSP features are implemented via handwritten tree traversal, these query files would still be valuable:

- `queries/tags.scm` for symbols and outline-like extraction
- `queries/brackets.scm` for Zed/editor bracket matching
- `queries/outline.scm` if editor-side structure should not depend on LSP
- `queries/locals.scm` only if local-scope capture patterns become useful for tooling

The current bindings already anticipate some of this expansion by conditionally exposing additional query constants when files exist.

## Semantic notes worth designing explicitly

- `!identifiers` should be modeled before rename and identifier completion are treated as stable.
- The server should report syntax diagnostics first and only layer on semantic diagnostics when the local parse tree is trustworthy.
- The Zed extension already owns some editor-specific query surfaces today, so not every useful query needs to live in this repository.

## Recommended order of feature delivery

### Phase A: useful with only parse trees plus light indexing

- syntax diagnostics
- keyword/directive completion
- document symbols
- basic go to definition

### Phase B: value unlocked by workspace graph

- references
- unresolved/duplicate diagnostics
- identifier completion
- include-aware indexing

### Phase C: polish and ecosystem fit

- rename
- richer hover
- workspace symbols
- semantic tokens
- code actions

The concrete post-MVP sequencing and guardrails for these later features are captured in `docs/lsp/03-delivery/advanced-semantic-expansion.md`.

## Guardrail

If a feature requires reproducing too much upstream execution behavior, it should be de-scoped unless it clearly improves editor workflows.
