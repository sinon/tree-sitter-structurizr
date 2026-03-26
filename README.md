# tree-sitter-structurizr

Tree-sitter grammar for the [Structurizr DSL](https://docs.structurizr.com/dsl/language).

The project goal is editor tooling first: syntax highlighting, folding, indentation, and robust parsing for real-world `.dsl` files. It is not trying to execute Structurizr scripts or provide full semantic validation.

## Status

The grammar is usable today for a meaningful subset of the DSL and has a fixture-first Rust test harness plus an upstream audit harness.

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

- The remaining broad umbrella sample from the upstream audit: `big-bank-plc.dsl`.
- Query authoring for highlighting/folding/indentation is still placeholder-only.

## Explicitly unsupported

These are currently out of scope on purpose rather than merely unfinished:

- `!script`
- `!plugin`

`!script` and `!plugin` are intentionally treated as unsupported because this grammar targets editor parsing, not executable DSL extensions. The contributor-only upstream audit excludes script and plugin-related fixtures by default so they do not block progress on the parser.

Upstream fixtures whose names contain `unexpected-` are also ignored permanently by the audit because they are intentional negative parser tests from the upstream project rather than valid DSL samples.

The audit also ignores `multi-line-with-error.dsl` permanently because it is an intentional invalid multiline sample whose remaining failure is the nested invalid model shape rather than the line-continuation syntax itself.

For local development commands, contributor-only audit workflow details, and repository layout notes, see [`CONTRIBUTORS.md`](./CONTRIBUTORS.md).
