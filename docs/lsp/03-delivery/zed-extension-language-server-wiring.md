# Structurizr DSL Zed extension language-server wiring

This note turns Phase 5 of `docs/lsp/03-delivery/roadmap.md` into a concrete integration plan for `/Users/rob/dev/zed-structurizr`.

Its job is to answer:

- what has to change in the existing Zed extension repo
- how the extension should launch the `strz` binary with the `server` subcommand
- how local development should work without slowing grammar/LSP iteration
- how published extension packaging should work without bundling the server into the extension repository

## Why this note exists

The planning work already established:

- grammar + analysis + LSP should stay in this repository
- the Zed extension should remain a separate downstream consumer
- Tree-sitter-native editor behavior should stay editor-native where possible

What still needed to be made explicit was the downstream wiring story:

- manifest shape
- extension-side Rust/Wasm shape
- binary discovery and download policy
- release and local-dev choreography

Without that, “wire it into Zed” stays vague and tends to drift into accidental packaging decisions.

## Current starting point

Today `/Users/rob/dev/zed-structurizr` is a static language extension:

- `extension.toml` registers the grammar
- `languages/structurizr/config.toml` defines the language metadata
- `languages/structurizr/` owns Zed-side queries
- there is currently **no** `Cargo.toml`
- there is currently **no** `src/lib.rs`
- there is currently **no** `language_server_command` implementation

That means the current extension can already provide:

- highlighting
- folds
- indentation
- outline
- brackets
- text objects

But it cannot yet launch a language server.

## Main conclusion

The Zed extension should stay a **thin launcher and packaging layer**.

It should:

1. keep consuming this repository as its pinned grammar source
2. add one registered language server for **Structurizr DSL**
3. add the minimum extension-side Rust/Wasm code needed to resolve and launch `strz server`
4. keep Tree-sitter-native editor features in Zed query files
5. treat the server binary as an external artifact that is either:
   - explicitly overridden for local development
   - found on the user’s `PATH`
   - or downloaded from a GitHub release

The extension should **not** try to absorb analysis logic or become the semantic source of truth.

The broader artifact/release lifecycle that sits around this launcher behavior is captured in `docs/lsp/03-delivery/packaging-and-dev-loop.md`.

## Responsibility split at wiring time

### This repository

Owns:

- grammar source and generated parser artifacts
- Rust parser crate
- `structurizr-analysis`
- `structurizr-lsp`
- release assets for the `strz` binary

### `/Users/rob/dev/zed-structurizr`

Owns:

- grammar pin in `extension.toml`
- language server registration in `extension.toml`
- extension-side Rust/Wasm launcher code
- binary resolution/download policy
- Zed-specific query files and editor tuning

This keeps the semantic logic close to the grammar while keeping editor-distribution details in the editor-facing repo.

## What should change in the Zed extension repo

The extension repo should gain exactly three new pieces:

1. a language-server registration in `extension.toml`
2. a small Rust extension crate (`Cargo.toml` + `src/lib.rs`)
3. a documented local-dev and release workflow

Nothing about the current query ownership decision should change:

- `outline.scm`
- `brackets.scm`
- `textobjects.scm`

stay extension-owned.

## Manifest shape

The existing manifest should keep the grammar registration:

```toml
[grammars.structurizr]
repository = "https://github.com/sinon/tree-sitter-structurizr"
rev = "..."
```

And it should add one language-server registration:

```toml
[language_servers.structurizr-lsp]
name = "Structurizr LSP"
languages = ["Structurizr DSL"]
```

Important details:

- `languages` must match the `name` in `languages/structurizr/config.toml`
- the language-server ID should stay stable and predictable
- the language-server ID can remain `structurizr-lsp` even though the launched executable is `strz`
- no second “syntax-only” language registration is needed

## Extension crate shape

Because Zed expects procedural extension logic to be written in Rust and compiled to WebAssembly, `zed-structurizr` should add a small extension crate.

Recommended shape:

```text
zed-structurizr/
  extension.toml
  Cargo.toml
  src/
    lib.rs
  languages/
    structurizr/
      config.toml
      highlights.scm
      folds.scm
      indents.scm
      outline.scm
      brackets.scm
      textobjects.scm
```

Recommended `Cargo.toml` shape:

```toml
[package]
name = "zed-structurizr"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7"
```

And `src/lib.rs` should stay intentionally small:

- define a single extension type
- implement `zed::Extension`
- implement `language_server_command`
- optionally later implement `language_server_workspace_configuration`
- register the extension with `zed::register_extension!`

This crate should not contain Structurizr semantic logic.

## Recommended launcher behavior

The extension should resolve the `strz` binary using a clear priority order.

## Resolution order

### 1. Explicit local Zed override

First, look for Zed's native binary override setting:

- `lsp.structurizr-lsp.binary.path`

This is the cleanest local-development hook because it:

- uses Zed's documented binary-override surface
- avoids editing extension source for every local experiment
- allows the contributor to point at `target/debug/strz`
- keeps local server selection deterministic even if another binary exists on `PATH`

Recommended local use:

```json
{
  "lsp": {
    "structurizr-lsp": {
      "binary": {
        "path": "/Users/rob/dev/tree-sitter-structurizr/target/debug/strz"
      }
    }
  }
}
```

The extension should treat this override as intentionally highest priority.

### 2. One-shot shell override

For terminal-launched local experiments, also honor:

- `STRUCTURIZR_LSP_BIN` from `worktree.shell_env()`

This keeps one-off smoke tests convenient without requiring a settings edit.
Because the override comes from Zed's shell environment, it is most reliable
when launching a fresh Zed instance rather than handing off to one that is
already running.

Recommended local use:

```sh
STRUCTURIZR_LSP_BIN=/Users/rob/dev/tree-sitter-structurizr/target/debug/strz zed --foreground
```

### 3. User-installed binary on `PATH`

If no explicit override exists, check:

- `worktree.which("strz")`

This supports:

- contributors who install the binary manually
- advanced users who want to manage the server themselves
- simple early dogfooding before download logic exists

This should be the first published fallback because it is transparent and easy to reason about.

### 4. Downloaded release asset

If no override exists and the binary is not on `PATH`, the published extension should download a platform-specific release artifact.

This should use the Zed extension API surface for:

- release lookup
- downloading into the extension working directory
- extracting archives
- marking the resulting file executable
- reporting installation status

That means using:

- `github_release_by_tag_name(...)` or later `latest_github_release(...)`
- `download_file(...)`
- `make_file_executable(...)`
- `set_language_server_installation_status(...)`

## Why this order

This order gives the best trade-off:

- local work stays fast
- published installs still work for users who do not want to manage binaries manually
- the extension never needs to pretend the server is part of the Wasm bundle

## Published extension strategy

The published extension should use a **pinned server release tag**, not “always latest”.

## Recommendation

For each extension release:

- pin a grammar commit in `extension.toml`
- pin an LSP release tag in the extension Rust code

Example conceptual pairing:

- grammar rev: `abc123...`
- LSP tag: `v0.4.0`

Why pinned is better than latest:

- reproducible extension behavior
- tested grammar/server pairing
- easier rollback if a release asset is broken
- less surprising behavior for users and maintainers

`latest_github_release(...)` can remain a later option for preview channels or explicit update checks, but it should not be the first production policy.

## Recommended release-asset source

Because the `strz` launcher is built from this repository, the extension should download release assets from:

- the same GitHub repository as the grammar/LSP source

That keeps:

- release provenance simple
- grammar/LSP changelog coordination simpler
- ownership boundaries clearer than introducing a fourth distribution repository

## Recommended asset naming

Pick one stable naming convention and keep it boring.

For example:

- `strz-macos-aarch64.tar.gz`
- `strz-macos-x86_64.tar.gz`
- `strz-linux-x86_64.tar.gz`
- `strz-windows-x86_64.zip`

The extracted executable should be:

- `strz` on macOS/Linux
- `strz.exe` on Windows

The extension launcher should map Zed’s `Os` and `Architecture` values onto this asset naming scheme directly.

## Failure and status policy

If the extension needs to download the binary:

- set installation status to `Downloading`
- on failure, set installation status to `Failed(...)`
- on success, return the final executable command cleanly

Do not silently swallow download or extraction failures.

If the extension cannot find or install the binary, it should fail with a direct message that tells the user which resolution steps are supported.

## Minimal extension-side Rust responsibilities

The first extension-side Rust code should stay narrow.

### Required now

- `language_server_command`

### Reasonable soon after

- `language_server_workspace_configuration`

### Not required initially

- `label_for_completion`
- `label_for_symbol`
- semantic-token customization
- any assistant-specific annotation behavior

The extension should launch `strz` with `server` first before trying to add protocol-result presentation polish.

## Workspace configuration policy

The extension should avoid inventing a large settings surface before the LSP actually has stable knobs.

Recommended bounded policy:

- `language_server_initialization_options`: return `None` initially
- `language_server_workspace_configuration`: return `None` initially

Only introduce explicit config once the server has genuine settings that matter, such as:

- future remote-include policy
- future trace/log level
- future semantic-token preferences

This keeps the extension from freezing an unnecessary config contract too early.

## What Zed should continue doing natively

The LSP should add semantic value, not replace the editor’s Tree-sitter layer.

Zed should continue to rely on extension-side Tree-sitter assets for:

- syntax highlighting
- folding
- indentation
- outline
- brackets
- text objects

The LSP should initially add:

- diagnostics
- document symbols
- go-to-definition
- find references
- bounded keyword/directive completion

Semantic tokens should remain a later polish decision, not part of the first wiring milestone.

## Local development workflow

The local workflow should optimize for changing grammar and server code together without committing every experiment.

## Recommended loop

1. build the local server binary in this repository
2. point the Zed extension at the local grammar repo with a `file://` grammar URL
3. configure `lsp.structurizr-lsp.binary.path` to point at the local `strz` binary
4. install `zed-structurizr` as a dev extension
5. open representative `.dsl` files and inspect logs in foreground mode

Concrete example:

1. in `/Users/rob/dev/tree-sitter-structurizr`, build `target/debug/strz`
2. in `/Users/rob/dev/zed-structurizr/extension.toml`, temporarily use:

```toml
[grammars.structurizr]
repository = "file:///Users/rob/dev/tree-sitter-structurizr"
rev = "<local-commit-sha>"
```

3. in Zed user settings or the target worktree's `.zed/settings.json`, add:

```json
{
  "lsp": {
    "structurizr-lsp": {
      "binary": {
        "path": "/Users/rob/dev/tree-sitter-structurizr/target/debug/strz"
      }
    }
  }
}
```

4. launch:

```sh
zed --foreground
```

5. install the dev extension from `/Users/rob/dev/zed-structurizr`

For one-shot terminal launches, step 3 can be replaced with:

```sh
STRUCTURIZR_LSP_BIN=/Users/rob/dev/tree-sitter-structurizr/target/debug/strz zed --foreground
```

If Zed is already running, prefer the settings-based override instead of the
shell-based one.

Important rule:

- never commit the `file://` grammar override

That override is only for local iteration.

If Zed requires a concrete revision even for `file://` grammar sources, keep that value local as well and treat it as part of the dev-only override.

## Manual smoke-test set

The first Zed wiring milestone should be validated against both extension-native and LSP-added behavior.

### Extension-native checks

Use `/Users/rob/dev/zed-structurizr/big-bank.dsl` to confirm:

- language detection still works
- highlighting still works
- folding/indentation still work
- outline still works
- bracket matching still works

### LSP-focused checks

Use the fixture slice in this repository to confirm:

- `tests/fixtures/lsp/identifiers/direct-references-ok.dsl`
  - bounded definition works for assigned identifiers
- `tests/fixtures/lsp/relationships/named-relationships-ok.dsl`
  - named relationship navigation works when implemented
- `tests/fixtures/lsp/includes/workspace_fragments-ok.dsl`
  - include-aware diagnostics resolve against the parent directive site
- `tests/fixtures/lsp/directives/identifiers-directive-ok.dsl`
  - hierarchical identifier mode does not crash or produce overconfident results

### Logging

For extension debugging:

- run Zed with `--foreground`
- use `zed: open log`

This should remain the default debug loop unless the extension later gains more specialized logging hooks.

## Release choreography

The release flow should keep grammar and LSP explicit instead of pretending there is one indivisible artifact.

## Recommended flow

1. land grammar/LSP changes in this repository
2. cut or update an LSP release tag and platform assets here
3. decide whether the Zed extension also needs a newer grammar rev
4. update the Zed extension:
   - grammar `rev`
   - pinned LSP tag constant in extension code
   - extension version
5. smoke-test locally
6. publish/update the Zed extension

## Why separate grammar rev and LSP tag

This is the most important release-detail decision in the note.

Even though grammar and LSP live in the same source repository, the extension should still treat them as two explicit pins:

- grammar rev controls Tree-sitter syntax assets
- LSP tag controls the downloaded executable

That gives the project room to ship:

- grammar-only updates
- LSP-only updates
- or matched updates

without forcing needless lockstep churn in every case.

## Recommended first implementation boundary

The first concrete Zed implementation should stop once these are true:

- the extension can launch `strz server`
- grammar loading still works from the pinned grammar repo
- local dev can use `file://` plus a local binary override
- Zed continues to use extension-owned queries for outline/brackets/textobjects
- the bounded MVP LSP features work without broader protocol polish

Do not block this milestone on:

- semantic tokens
- rename
- hover richness
- extension-managed auto-updates
- full user-facing configuration UI

## What this unblocks

Once this wiring note is followed:

- the LSP has a concrete downstream editor target
- local grammar/LSP/editor iteration remains fast
- published extension packaging has a clear path that matches Zed’s extension model
- the grammar repo and Zed repo can stay loosely coupled without becoming disconnected

## Sources

- `docs/lsp/01-foundations/repository-topology.md`
- `docs/lsp/01-foundations/query-ownership.md`
- `docs/lsp/02-design/lsp-crate-skeleton.md`
- `docs/lsp/02-design/bounded-mvp-handlers.md`
- `/Users/rob/dev/zed-structurizr/extension.toml`
- `/Users/rob/dev/zed-structurizr/languages/structurizr/config.toml`
- `/Users/rob/dev/zed-structurizr/README.md`
- `https://zed.dev/docs/extensions/languages`
- `https://zed.dev/docs/extensions/developing-extensions`
- `https://zed.dev/docs/extensions/capabilities`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/trait.Extension.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/process/struct.Command.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/struct.Worktree.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/fn.download_file.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/fn.make_file_executable.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/enum.LanguageServerInstallationStatus.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/enum.DownloadedFileType.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/fn.github_release_by_tag_name.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/fn.latest_github_release.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/enum.Os.html`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/enum.Architecture.html`
