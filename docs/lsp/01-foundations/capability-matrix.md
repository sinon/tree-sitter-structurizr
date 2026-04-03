# Structurizr DSL LSP capability matrix

This matrix records which editor features are already shipped in-repo, which ones are the next steps toward a feature-complete experience, and which ones remain deliberate later work.

## Status key

- **Shipped**: implemented in the current in-repo analysis/LSP stack
- **Shipped, bounded**: implemented, but intentionally conservative around deferred scope cases
- **Next**: high-value follow-on work before the stack feels feature-complete
- **Later**: valuable, but not on the shortest path to a complete-feeling editor experience
- **Out of scope**: intentionally outside the goals of this repository

## Feature matrix

| Feature                                                                  | Primary layer                | Status           | Notes                                                                                                      |
| ------------------------------------------------------------------------ | ---------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------- |
| Syntax diagnostics                                                       | Tree-sitter + analysis + LSP | Shipped          | Parse errors already flow through the current snapshots and handlers.                                      |
| Include diagnostics                                                      | Analysis + LSP               | Shipped          | Missing and cyclic file-resolution cases are already surfaced at directive sites.                          |
| Bounded semantic diagnostics                                             | Analysis + LSP               | Shipped, bounded | Current duplicate and ambiguous identifier cases are handled for the shipped symbol families.              |
| Document symbols                                                         | Analysis + LSP               | Shipped          | Powered by structural symbol facts rather than editor-only outline logic.                                  |
| Keyword/directive completion                                             | Syntax + LSP                 | Shipped          | Fixed-vocabulary completion already works.                                                                 |
| Style-property completion                                                | Syntax + LSP                 | Shipped          | Property-name completion is already landed for style blocks.                                               |
| Directive-path document links                                            | Syntax + LSP                 | Shipped          | `textDocument/documentLink` is supported, with a `definition` fallback for clients that need file targets. |
| Go to definition                                                         | Analysis + LSP               | Shipped, bounded | Core cross-file navigation already works for the currently modeled symbol families.                        |
| Find references                                                          | Analysis + LSP               | Shipped, bounded | Shares the same bounded semantic model as definition.                                                      |
| Type definition                                                          | Analysis + LSP               | Shipped, bounded | Instance-to-model navigation is already exposed through `textDocument/typeDefinition`.                     |
| Broader scope/reference coverage (`this`, selectors, named dynamic refs) | Analysis + LSP               | Next             | This is the biggest remaining gap inside the current semantic model.                                       |
| Identifier completion                                                    | Analysis + LSP               | Next             | Needs broader scope confidence and clearer `!identifiers` policy.                                          |
| Hover                                                                    | Analysis + LSP               | Shipped, bounded | Current hover covers the bounded identifier families with compact source-derived metadata from declarations. |
| Workspace symbols                                                        | Analysis + LSP               | Next             | A good follow-on once workspace facts cover more symbol families.                                          |
| Rename                                                                   | Analysis + LSP               | Later            | Should wait for broader reference coverage and conflict checks.                                            |
| Semantic tokens                                                          | Analysis + LSP               | Later            | Nice polish, but not core value compared with stronger navigation and diagnostics.                         |
| Code actions                                                             | Analysis + LSP               | Later            | Best added after diagnostics and rename are trustworthy.                                                   |
| Runtime-style validation                                                 | Runtime                      | Out of scope     | The project stays editor-oriented rather than runtime-oriented.                                            |

## What should stay query-owned

Tree-sitter queries and editor-native behavior should continue to own as much of the pure syntax experience as possible.

That includes:

- highlighting
- folding
- indentation
- bracket behavior where the editor already supports it cleanly
- editor-specific query surfaces that do not need semantic resolution

The LSP should add semantic value, not duplicate what the grammar/query layer already does well.

## What still makes the stack feel incomplete

The current gaps are less about whether the server exists and more about how complete the current semantic coverage feels in practice.

The biggest missing pieces are:

- broader safe resolution across the still-deferred scope families
- richer read-only semantic feedback such as hover and workspace symbols
- safe edit-capable features such as rename
- downstream packaging and editor wiring that make the current in-repo server easy to consume

For sequencing and delivery detail, continue with [`../03-delivery/roadmap.md`](../03-delivery/roadmap.md).
