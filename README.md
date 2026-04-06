# tree-sitter-structurizr

Structurizr editor tooling built around the `strz` language server and a Tree-sitter grammar for `.dsl`.

## What are you here for?

- **Use Structurizr in Zed** -> start with [Using `strz` with Zed today](#using-strz-with-zed-today)
- **Parse Structurizr DSL directly in Rust** -> jump to [Using the grammar directly](#using-the-grammar-directly)
- **Contribute to the grammar, analysis layer, or LSP** -> start with [`CONTRIBUTING.md`](./CONTRIBUTING.md)
- **Understand the current LSP architecture and roadmap** -> start with [`docs/lsp/README.md`](./docs/lsp/README.md)

## Using `strz` with Zed today

Today the most reliable setup is local and explicit: install a Rust toolchain, build `strz`, then point Zed at that binary.

1. Build the binary:

```sh
cargo build -p strz --bin strz --release
```

2. Verify the binary works:

```sh
./target/release/strz check your-workspace.dsl
./target/release/strz server
```

3. Install the [`zed-structurizr`](https://github.com/sinon/zed-structurizr) extension.

> [!NOTE]
> The extension currently needs to be installed manually as it is pre-release.
> In the future the extension will download `strz` if not installed on the system.

4. Point Zed at it:

```json
{
  "lsp": {
    "strz-lsp": {
      "binary": {
        "path": "~/path/to/tree-sitter-structurizr/target/release/strz"
      }
    }
  }
}
```

### What works today

- syntax, include, and bounded semantic diagnostics
- document symbols
- keyword/directive and style-property completion
- hover for the current bounded identifier families, with source-derived metadata summaries
- go-to-definition, find-references, and type-definition for the currently supported reference shapes
- directive path links and path-opening fallbacks

### What is still intentionally conservative

- `this`
- selector-style references such as `system.api`
- named dynamic relationship reference sites
- rename, workspace symbols, semantic tokens, and code actions

For deeper status, delivery, and configuration detail, continue with:

- [`docs/lsp/00-current-state.md`](./docs/lsp/00-current-state.md)
- [`docs/lsp/03-delivery/roadmap.md`](./docs/lsp/03-delivery/roadmap.md)
- [`docs/lsp/03-delivery/zed-extension-language-server-wiring.md`](./docs/lsp/03-delivery/zed-extension-language-server-wiring.md)

## Using the grammar directly

If you only want syntax parsing or the Rust grammar crate, start here:

```toml
[dependencies]
tree-sitter = "0.26.7"
tree-sitter-structurizr = "0.0.1"
```

```rust
let code = r#"
workspace {
    model {
    }

    views {
    }
}
"#;

let mut parser = tree_sitter::Parser::new();
let language = tree_sitter_structurizr::LANGUAGE;
parser
    .set_language(&language.into())
    .expect("Error loading Structurizr parser");

let tree = parser.parse(code, None).unwrap();
assert!(!tree.root_node().has_error());
```

For grammar coverage details, test surfaces, and contributor workflow, start with [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## Contributing

For grammar, analysis, LSP, benchmarking, and release workflow, start with [`CONTRIBUTING.md`](./CONTRIBUTING.md).

For the current LSP architecture, status, and doc map, start with [`docs/lsp/README.md`](./docs/lsp/README.md).

## License and upstream provenance

Original code in this repository is available under either the MIT License ([`LICENSE-MIT`](LICENSE-MIT)) or the Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE)).

This repository also includes material copied or adapted from the Apache-2.0 licensed Structurizr DSL project in [`structurizr/structurizr`](https://github.com/structurizr/structurizr), predominantly consisting of checked-in `.dsl` samples and fixtures.

These consist of:

- Structurizr DSL corpus material under [`crates/strz-grammar/test/corpus/`](crates/strz-grammar/test/corpus/)
- General Structurizr DSL fixtures under [`fixtures/`](fixtures/)
- LSP-specific single-document fixtures under [`crates/strz-lsp/tests/fixtures/`](crates/strz-lsp/tests/fixtures/)
- Multi-file workspace fixtures under [`tests/lsp/workspaces/`](tests/lsp/workspaces/)

## References

- Structurizr DSL overview: <https://docs.structurizr.com/dsl>
- Structurizr DSL language reference: <https://docs.structurizr.com/dsl/language>
- Upstream Structurizr repository: <https://github.com/structurizr/structurizr>
