# Structurizr DSL formatting policy

> Status: planned for implementation.
>
> This note locks the first-pass contract for `strz format` so later code work
> can implement one canonical layout without re-deciding whitespace policy
> file by file.

This note covers formatting policy for one physical `.dsl` document plus the
workspace-level boundaries around batch formatting. It complements the
workspace discovery contract in
[`docs/lsp/02-design/workspace-discovery-includes.md`](./lsp/02-design/workspace-discovery-includes.md#structurizr-dsl-workspace-discovery-and-include-resolution).

## What the checked-in corpus already says

A scan of the repository's checked-in `.dsl` files shows a very strong default
style:

- 344 `.dsl` files
- no tabs
- no CRLF line endings
- no trailing whitespace
- opening braces on the same line as the header
- closing braces on their own lines
- ordinary DSL indentation overwhelmingly in multiples of 4 spaces
- local human-authored DSL sits at roughly `p95 ~= 81` columns once cursor
  fixtures, synthetic benchmark workspaces, and `tmp/` repros are excluded
- a small non-upstream GitHub sample showed a similar shape at roughly
  `p95 ~= 83` columns, with the longest real lines concentrated in declaration
  headers, workspace headers, include lists, and URL-heavy lines

The few counterexamples do not describe a competing house style:

- [`fixtures/model/leading-digit-identifiers-ok.dsl`](../fixtures/model/leading-digit-identifiers-ok.dsl)
  intentionally uses non-canonical 2-space and 6-space indentation for grammar
  coverage and should normalize if formatted.
- [`fixtures/views/text_blocks-ok.dsl`](../fixtures/views/text_blocks-ok.dsl)
  and
  [`fixtures/styles/comments_and_styles-ok.dsl`](../fixtures/styles/comments_and_styles-ok.dsl)
  contain raw text payloads whose interior spacing is part of the content, not
  part of the house style.
- [`fixtures/model/all-digit-identifiers-err.dsl`](../fixtures/model/all-digit-identifiers-err.dsl)
  is a negative parse fixture and should be rejected rather than reformatted.
- Files under `tmp/` are local repro artifacts, not formatter style inputs.

## Bounded goal

The first formatter should do only this:

1. normalize structural whitespace and blank lines
1. apply one canonical indentation and brace style
1. keep standalone fragments and workspace entry documents format-worthy
1. preserve local and remote include topology
1. refuse to rewrite parse-error documents

It should not try to:

- canonicalize keyword casing or synonym choice such as `softwareSystem` versus
  `softwaresystem`, `systemContext` versus `systemcontext`, or `color` versus
  `colour`
- rewrite directive values, include quoting, URLs, or identifier spelling
- flatten multi-file workspaces into one document
- fetch remote includes
- infer best-effort layout from recovery trees

## Canonical layout policy

### Indentation and line endings

- Use spaces only.
- The default indentation width is `4`.
- Store indentation width in code-level configuration, not CLI flags.
- Use LF line endings.
- Strip trailing horizontal whitespace.

### Line width

- Treat `110` columns as the default soft target for ordinary DSL lines.
- The target is best-effort rather than a hard cap. Unsplittable content such as
  long strings, long URLs, and preserved comments may exceed it.
- Because ordinary Structurizr statements are line-oriented, wrapped DSL
  statements should use explicit line-continuation markers (`\`) rather than
  relying on raw newline insertion alone.
- The first wrapping targets should be:
  - long workspace, view, element, deployment, and style headers
  - long relationship statements
  - long `include` and `themes` lists
- Prefer wrapping at syntactic field boundaries rather than splitting strings or
  identifiers.
- Keep the opening `{` on the final header line when wrapping a block header,
  unless doing so would still exceed the limit because the final field itself is
  unsplittable.

### Block layout

- Keep the opening `{` on the same line as the owning statement header.
- Put the closing `}` on its own line at the current block indentation.
- Use multi-line block layout even for empty blocks in v1.
- Print one ordinary DSL statement per physical line by default.

### Vertical spacing

- Do not keep leading or trailing blank lines inside a block.
- Separate top-level source-file definition blocks and top-level `workspace`
  sections with one blank line.
- Inside `views`, separate view definitions and major sibling sections such as
  `styles`, `themes`, `branding`, `terminology`, and `properties` with one
  blank line.
- Keep nested model, deployment, style, properties, and animation bodies
  compact by default rather than inserting blank lines between each sibling.
- Treat attached comments as layout barriers so comment groups stay visibly
  attached to the node they describe.

## Preservation boundaries

The formatter is layout-oriented, not a token canonicalizer.

It should preserve exactly:

- comment text, including block-comment interior whitespace
- text-block payloads such as embedded PlantUML, Markdown, or HTML
- ordinary string spelling and escape sequences
- accepted keyword spelling and casing variants
- explicit line-continuation spelling in v1

In practice that means the formatter should normalize indentation, braces, and
structural blank lines around DSL nodes, but it should not rewrite raw payloads
or replace one accepted token spelling with another.

## Comment handling

- Preserve comment text verbatim in v1 for `#`, `//`, and `/* */` comments.
- Do not reflow comment prose to satisfy the line-width target.
- Reindent comments with the surrounding block, but preserve the interior line
  structure of block comments and the exact trailing text of line comments.
- Treat the `110`-column target as advisory for comments rather than mandatory.

## Parse-error boundary

Only syntax errors block rewriting.

If a document contains parse recovery such as `ERROR` or `MISSING` nodes, v1
should refuse to rewrite it and surface that refusal clearly. Semantic or
workspace diagnostics do not, by themselves, block formatting.

This keeps the formatter conservative where the tree shape is already known to
be unreliable, while still allowing formatting for documents that are
syntactically valid but semantically incomplete.

## Workspace scope

Formatting remains document-oriented even when the command is workspace-aware.

When `strz format` is pointed at a directory or workspace root, it should:

1. discover local formatting targets deterministically
1. format every discovered local document independently
1. include fragment documents in that target set
1. skip remote include targets rather than fetching or rewriting them

It should not inline includes, merge workspace instances, or treat formatting as
a source-flattening feature.
