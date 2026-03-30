# tree-sitter-structurizr

Tree-sitter grammar and Rust bindings for the Structurizr DSL.

## Usage

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
    .expect("Structurizr parser should load");

let tree = parser.parse(code, None).expect("source should parse");
assert!(!tree.root_node().has_error());
```

For repository-level contributor workflow, LSP context, and downstream editor
integration notes, see the workspace root `README.md`.
