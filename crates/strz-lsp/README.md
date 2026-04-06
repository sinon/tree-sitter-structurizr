# strz-lsp

`strz-lsp` is the in-repo stdio language server for Structurizr DSL editor features.

It stays intentionally thin:

- `strz-analysis` owns extracted document and workspace facts
- this crate owns LSP capabilities, server state, handlers, and type conversion
- `strz server` is the standard binary entrypoint that launches it

## Current shipped surface

- full-document text sync
- diagnostics
- document symbols
- keyword/directive, style-property, and flat-mode relationship identifier
  completion for explicit core model relationship endpoints
- hover for the current bounded identifier families, rendered from source-derived symbol metadata
- definition, references, rename, and type-definition for the current bounded
  symbol families
- document links for directive paths

## Crate shape

- [`src/lib.rs`](src/lib.rs) - `serve_stdio()`
- [`src/capabilities.rs`](src/capabilities.rs) - advertised server capabilities
- [`src/server.rs`](src/server.rs), [`src/state.rs`](src/state.rs), [`src/documents.rs`](src/documents.rs) - runtime state and wiring
- [`src/handlers/`](src/handlers/) - request handlers
- [`src/convert/`](src/convert/) - analysis-to-LSP conversions

## Useful repo-root commands

```sh
cargo test -p strz-lsp --test lifecycle
cargo test -p strz-lsp --test hover
cargo test -p strz-lsp --test navigation
cargo test -p strz-lsp --test rename
cargo bench -p strz-lsp --bench session
just run-strz server
```

## Related docs

- [`../../docs/lsp/00-current-state.md`](../../docs/lsp/00-current-state.md) for shipped versus deferred behavior
- [`../../docs/lsp/01-foundations/overview.md`](../../docs/lsp/01-foundations/overview.md) for architecture boundaries
- [`../../docs/lsp/02-design/bounded-mvp-handlers.md`](../../docs/lsp/02-design/bounded-mvp-handlers.md) for the bounded handler contract
- [`../../docs/lsp/03-delivery/roadmap.md`](../../docs/lsp/03-delivery/roadmap.md) for remaining delivery work
