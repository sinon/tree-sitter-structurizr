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

## Resolution

Keep semantic symbol navigation and path-link opening on separate protocol surfaces.

- Keep semantic symbol navigation and `textDocument/references` limited to symbol usage, not plain path spans.
- Implement path opening for those spans via `textDocument/documentLink`.
- Also answer `textDocument/definition` for those spans as a practical Zed fallback, because Zed does not yet surface LSP document links.
- For directory-valued directives such as `!docs` and `!adrs`, make that fallback point at concrete files inside the directory because Zed expects file definition targets.

This keeps the dedicated link surface available for editors that support it while still giving Zed a concrete Cmd-click path-opening route today.

The current `zed_extension_api` only exposes language-server launch and configuration hooks, so there is no extension-native clickable-span hook to own this directly at the moment.
