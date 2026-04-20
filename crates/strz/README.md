# strz

`strz` provides the `strz` binary which is the CLI interface for `strz-analysis` and `strz-lsp`.

## Command surface

- `strz check [PATH ...]` - aggregated syntax, include, and bounded semantic diagnostics
  - defaults to the current directory when no path is provided
  - supports `--syntax-only`, `--include-only`, and `--warnings-as-errors`
  - includes semantic diagnostics by default; the filter flags stay strict
- `strz format [PATH ...]` - canonical Structurizr formatting for local files and discovered local workspace fragments
  - defaults to the current directory when no path is provided
  - rewrites local documents in place by default
  - supports `--check` to report whether formatting would change any discovered local documents
- `strz dump document <PATH>` - a single-document analysis snapshot
- `strz dump workspace [PATH ...]` - workspace discovery and include-following facts
- `strz version` - prints the package version plus compile-time build metadata
  - defaults to `unknown` for the Git SHA when built outside a repository checkout
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
just run-strz version
just run-strz format --check
just run-strz dump document crates/strz-lsp/tests/fixtures/identifiers/direct-references-ok.dsl
just run-strz dump workspace tests/lsp/workspaces/directory-include
just run-strz server
cargo test -p strz
```

## Related docs

- [`../../README.md`](../../README.md) for the consumer-first `strz` + Zed path
- [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) for contributor workflow and logging examples
- [`../../docs/lsp/README.md`](../../docs/lsp/README.md) for the broader analysis/LSP doc map
