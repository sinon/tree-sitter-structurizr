# structurizr-cli

`structurizr-cli` provides the `strz` binary.

Use it when you want to:

- run syntax and include checks without an editor
- inspect analysis-layer facts as text or JSON
- launch the same stdio LSP entrypoint that downstream editor integrations should execute

## Command surface

- `strz check [PATH ...]` - aggregated syntax and include diagnostics
  - defaults to the current directory when no path is provided
  - supports `--syntax-only`, `--include-only`, and `--warnings-as-errors`
- `strz dump document <PATH>` - a single-document analysis snapshot
- `strz dump workspace [PATH ...]` - workspace discovery and include-following facts
- `strz server` - runs the Structurizr LSP over stdio

Global flags:

- `--output-format text|json`
- `--color auto|always|never`
- `--quiet` / `--verbose`

## Observability

The CLI and server share the same opt-in logging controls:

- `RUST_LOG`
- `STRZ_LOG_FORMAT=compact|json`
- `STRZ_LOG_FILE=path`

## Useful repo-root commands

```sh
just build-strz
just run-strz check
just run-strz dump document crates/structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl
just run-strz dump workspace tests/lsp/workspaces/directory-include
just run-strz server
cargo test -p structurizr-cli
```

## Related docs

- [`../../README.md`](../../README.md) for the consumer-first `strz` + Zed path
- [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) for contributor workflow and logging examples
- [`../../docs/lsp/README.md`](../../docs/lsp/README.md) for the broader analysis/LSP doc map
