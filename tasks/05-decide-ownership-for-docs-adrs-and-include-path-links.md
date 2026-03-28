## Issue

The copied Big Bank workspace raises two path-navigation expectations that do not fit neatly into the current bounded LSP definition contract:

- `!docs` and `!adrs` arguments should open the related folder
- top-level interpolated `!include` paths should open the included file instead of falling back to editor search

These behaviors may belong to editor-native file linking rather than semantic symbol navigation.

## Root Cause

The current LSP roadmap does not plan `textDocument/documentLink`, and `docs/lsp/02-design/bounded-mvp-handlers.md` only treats `ViewInclude` references as navigation-capable inside the bounded definition handler.

That leaves two open questions:

- should folder/file-path navigation be owned by downstream Zed behavior instead of the LSP
- if the LSP should own it, should it use `documentLink` rather than overloading `gotoDefinition`

The interpolated `!include` case is especially ambiguous because it may be an editor-routing issue rather than a server-resolution issue.

## Options

- Keep folder and include-path navigation editor-owned in `zed-structurizr`, using Tree-sitter-aware file-link behavior there.
- Add an LSP `documentLink` surface in this repo for directive arguments that resolve to folders or files.
- Split ownership: keep semantic symbol navigation in the LSP, while editor-local path links stay downstream.

## Proposed Option

Resolve ownership first, not implementation. The likely default should be to keep plain path links editor-owned unless there is a strong cross-editor case for LSP support.

If LSP support is later chosen, prefer `textDocument/documentLink` for `!docs`, `!adrs`, and top-level include paths instead of stretching `textDocument/definition` beyond its current semantic role.
