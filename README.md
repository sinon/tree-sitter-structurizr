# tree-sitter-structurizr

Tree-sitter grammar for the [Structurizr DSL](https://docs.structurizr.com/dsl/language).

The project goal is editor tooling first: syntax highlighting, folding, indentation, and robust parsing for real-world `.dsl` files. It is not trying to execute Structurizr scripts or provide full semantic validation.

## Status

The grammar is usable today for a meaningful subset of the DSL and has a Rust-first test harness plus an upstream audit harness.

Current validation:

- `tree-sitter test`
- `cargo nextest run`
- `cargo test --doc`
- `cargo test --test upstream_audit -- --ignored --nocapture`

## Supported today

The following syntax is implemented and covered by the local corpus and Rust test suite:

- Workspace structure: `workspace`, nested `model`, `views`, and `configuration` blocks.
- Core metadata and tokens: strings, numbers, identifiers, wildcard values, and comments (`//`, whitespace-prefixed `#`, and `/* ... */`).
- Core model elements: `person`, `softwareSystem`, `container`, and `component`, including identifier assignment.
- Relationships: basic `->`, tagged operators such as `--https->`, `this`, and relationship bodies used by the current fixtures.
- Views: `systemLandscape`, `systemContext`, `container`, `component`, `filtered`, `dynamic`, `deployment`, `custom`, and `image`.
- Common view statements: `include`, `exclude`, `autoLayout`, `default`, `title`, and `description`.
- Styles inside `views`: `styles`, `element`, `relationship`, and flat style settings like `background`, `shape`, `color`, and `opacity`.
- Directives and configuration currently used by fixtures: `!include`, `!identifiers`, `!impliedRelationships`, `!docs`, `!adrs`, plus `configuration { scope, visibility, users }`.
- Initial archetype/custom-element support: `archetypes`, custom elements, basic archetype extension by identifier, `!elements`, and `!element`.

## Not yet implemented

These areas are still in progress. Some parse partially, but they are not considered complete or stable yet:

- Richer archetype defaults and extensions, especially nested defaults such as `properties` and `perspectives` inside archetype definitions.
- Full group semantics and broader model-level metadata/property constructs.
- Broader workspace extension/include coverage beyond the currently tested directive forms.
- Remaining dynamic/deployment edge cases found in upstream fixtures.
- Additional relationship/filter expression variants from upstream examples.
- Text block and multiline constructs not covered by the current fixtures.
- Query authoring for highlighting/folding/indentation is still placeholder-only.

## Explicitly unsupported

These are currently out of scope on purpose rather than merely unfinished:

- `!script`

`!script` is intentionally treated as unsupported because this grammar targets editor parsing, not executable DSL extensions. The upstream audit excludes script-related fixtures by default so they do not block progress on the parser.

Upstream fixtures whose names contain `unexpected-` are also ignored permanently by the audit because they are intentional negative parser tests from the upstream project rather than valid DSL samples.

If you want to include those fixtures in an audit run anyway, use:

```sh
just audit-upstream-all
```

or:

```sh
STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 just audit-upstream
```

## Commands

The `Justfile` is the canonical workflow surface:

```sh
just generate
just test-grammar
just test-rust
just audit-upstream
```

Useful variants:

- `just test-rust-fast` runs `cargo nextest run` without doctests.
- `just audit-upstream-all` includes explicitly unsupported upstream fixtures such as `!script`.
- `STRUCTURIZR_UPSTREAM_FILTER=<text> just audit-upstream` narrows the upstream audit to matching file paths.

## Repository layout

- `grammar.js` defines the grammar.
- `src/parser.c`, `src/grammar.json`, and `src/node-types.json` are generated artifacts and should be regenerated after grammar changes.
- `tests/` contains the Rust parser harness, fixtures, snapshots, and upstream audit.
- `test/corpus/` contains Tree-sitter CLI corpus tests.
- `queries/` is reserved for editor query files used by consumers like Zed.

## Development approach

The project is being built in tiers:

1. Core workspace/model/view structure.
2. Core model elements and relationships.
3. View coverage.
4. Advanced directives, configuration, deployment/image/custom views.
5. Archetypes and upstream-fixture-driven hardening.

The upstream audit is the main tool for discovering the next missing grammar slices.
