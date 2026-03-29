# tree-sitter-structurizr

Tree-sitter grammar for the [Structurizr DSL](https://docs.structurizr.com/dsl/language).

This repository focuses on practical, test-backed coverage of the Structurizr DSL for Tree-sitter consumers. The scope is the grammar, queries, and bindings surface for real `.dsl` files rather than executable DSL extensions or full semantic validation.

## Status

The grammar is already useful for a meaningful subset of the DSL, with:

- checked-in generated parser artifacts
- Tree-sitter corpus tests
- Rust fixture and snapshot tests
- an upstream audit harness for coverage hardening

Current release status should be read as **early and iterating** rather than feature-complete. The parser is intentionally being expanded in slices, with coverage driven by local tests and upstream examples.

## Shipped surface

Today this repository ships:

- the Tree-sitter grammar source in `grammar.js`
- generated parser artifacts in `src/`
- Rust bindings in `bindings/rust/`
- checked-in query files in `queries/`

Current binding availability from `tree-sitter.json`:

- Rust: shipped
- C, Go, Java, Node, Python, Swift, Zig: not currently shipped in this repository

The query files are **real, checked-in editor-support artifacts**, not empty placeholders. They already cover highlighting, folding, and indentation for the syntax families implemented today, but they are still incomplete relative to the full Structurizr DSL.

## Using from Rust

Add the parser alongside `tree-sitter`:

```toml
[dependencies]
tree-sitter = "0.26.7"
tree-sitter-structurizr = "0.0.1"
```

Then load the language into a parser:

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

For deeper contributor workflow and development commands, start with [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## `strz` CLI

The Rust workspace now also includes `strz`, a contributor-facing CLI on top of
`structurizr-analysis` and the in-repo LSP server.

It is useful when you want to verify syntax and workspace include behavior
without launching the LSP through an editor:

```sh
cargo run -p structurizr-cli --bin strz -- check
cargo run -p structurizr-cli --bin strz -- dump workspace tests/lsp/workspaces/directory-include
cargo run -p structurizr-cli --bin strz -- dump document tests/fixtures/lsp/identifiers/direct-references-ok.dsl
cargo run -p structurizr-cli --bin strz -- server
```

That development command surface matches the installed binary shape:
`strz check`, `strz dump`, and `strz server`.

The CLI supports both human-oriented text output and `--output-format json`,
making it suitable for local debugging, snapshots, future CI-style semantic
checks, and editor integration.

## Supported today

The following syntax is implemented and covered by the local corpus and Rust test suite:

- Workspace structure: `workspace`, nested `model`, `views`, and `configuration` blocks.
- Core metadata and tokens: strings, numbers, identifiers, wildcard values, and comments (`//`, whitespace-prefixed `#`, and `/* ... */`).
- Core model elements: `person`, `softwareSystem`, `container`, and `component`, including identifier assignment.
- Deployment model constructs: `deploymentEnvironment` with or without a body, `deploymentGroup`, `deploymentNode`, `infrastructureNode`, `containerInstance`, `softwareSystemInstance`, and `instanceOf`, including instance bodies with relationships to `this`.
- Relationships: basic `->`, `-/>`, tagged operators such as `--https->`, assigned relationships like `r = a -> b`, `this`, relationship bulk updates via `!relationships`, and relationship bodies used by the current fixtures.
- Views: `systemLandscape`, `systemContext`, `container`, `component`, `filtered`, `dynamic`, `deployment`, `custom`, and `image`, plus `branding` and `terminology`.
- Common view statements: `include`, `exclude`, `animation`, `autoLayout`, `default`, `title`, `description`, and per-view `properties`.
- Deployment/view helpers used by current fixtures: `animation`, `theme`, `themes`, image `light`/`dark` source groups, and lowercase `autolayout`.
- Dynamic view coverage includes explicit relationship references such as `r2 "Async"`, nested parallel blocks, and no-description relationship bodies for nested flows or metadata.
- Styles inside `views`: `styles`, `element`, `relationship`, light/dark style modes, inline `theme`/`themes`, and flat style settings like `background`, `shape`, `color`, and `opacity`.
- Directives and configuration currently used by fixtures: `!include` at workspace and model level, `!const`, `!constant`, `!var`, `!identifiers`, `!impliedRelationships`, `!docs`, `!adrs`, `!elements`, `!element`, `!relationships`, workspace/model `properties`, plus `configuration { scope, visibility, users }`.
- Text features used by current fixtures: triple-quoted text blocks, multiline `\` continuations between tokens and inside quoted strings, and image/PlantUML sources fed from text blocks.
- Expanded archetype/custom-element support: archetype defaults, nested `properties` and richer `perspectives` inside archetype bodies, relationship archetype extensions such as `sync = -> { ... }` / `--sync->`, custom elements, `!elements`, `!element`, hierarchical selectors like `a.b.c`, deployment-node selectors, and selector updates inside nested groups.

## Not yet implemented

These areas are still in progress. Some parse partially, but they are not considered complete or stable yet:

- broader grammar coverage beyond the syntax families already represented in the local tests

## Explicitly unsupported

These are currently out of scope on purpose rather than merely unfinished:

- `!script`
- `!plugin`

`!script` and `!plugin` are intentionally treated as unsupported because this grammar targets editor parsing, not executable DSL extensions. The contributor-only upstream audit excludes script and plugin-related fixtures by default so they do not block progress on the parser.

Upstream fixtures whose names contain `unexpected-` are also ignored permanently by the audit because they are intentional negative parser tests from the upstream project rather than valid DSL samples.

The audit also ignores `multi-line-with-error.dsl` permanently because it is an intentional invalid multiline sample whose remaining failure is the nested invalid model shape rather than the line-continuation syntax itself.

## Contributing

Start with [`CONTRIBUTING.md`](./CONTRIBUTING.md) for contributor setup, canonical commands, and how the corpus and fixtures are organized.

## License and upstream provenance

Original code in this repository is available under either the MIT License
(`LICENSE-MIT`) or the Apache License, Version 2.0 (`LICENSE-APACHE`).

This repository also includes material copied or adapted from the Apache-2.0
licensed Structurizr DSL project in
[`structurizr/structurizr`](https://github.com/structurizr/structurizr),
predominently consisting of checked-in `.dsl` samples and fixtures.

These consist of:
- Structurizr DSL corpus material under `test/corpus/`
- Structurizr DSL fixtures and workspaces under `tests/fixtures/`
  and `tests/lsp/workspaces/`

## References

- Structurizr DSL overview: <https://docs.structurizr.com/dsl>
- Structurizr DSL language reference: <https://docs.structurizr.com/dsl/language>
- Upstream Structurizr repository: <https://github.com/structurizr/structurizr>
