# Structurizr DSL packaging and development loop

This note defines the recommended packaging model and contributor loops for the current Structurizr DSL grammar, analysis layer, LSP, and Zed extension.

It is the operational companion to:

- [`docs/lsp/01-foundations/repository-topology.md`](../01-foundations/repository-topology.md)
- [`docs/lsp/02-design/lsp-crate-skeleton.md`](../02-design/lsp-crate-skeleton.md)
- [`docs/lsp/03-delivery/zed-extension-language-server-wiring.md`](zed-extension-language-server-wiring.md)

Its job is to answer:

- which artifacts are source-only vs shipped
- how the extension should acquire the LSP in local development and in published releases
- how contributors should iterate on grammar-only, LSP-only, and integrated changes
- how release updates should flow without forcing needless lockstep between grammar, LSP, and extension

## Why this note exists

The wiring note already settled the launch-side behavior for Zed:

- local override first
- `PATH` second
- pinned GitHub release download for published installs

But that still leaves a broader operational question:

- what exactly are we packaging, and what are we not packaging?

This matters because the project has four different layers:

1. the Tree-sitter grammar/parser crate
1. the analysis crate
1. the shipped `strz` binary, which exposes the language server via `strz server`
1. the separate Zed extension repo

If those layers are not packaged deliberately, contributors will end up mixing:

- source dependencies
- editor packaging
- binary release assets
- dev-only overrides

in ways that are harder to test and harder to maintain.

## Main conclusion

The near-term model should be:

1. keep the grammar and Rust crates in this repository
1. treat the analysis crate as an internal source-level workspace member, not a separately distributed editor artifact
1. distribute `strz` as pinned GitHub release binaries from this repository
1. keep the Zed extension in `/Users/rob/dev/zed-structurizr` as a separate published package that:
   - pins a grammar commit
   - pins an LSP release tag
   - downloads or locates the LSP binary
1. keep local development fast with:
   - `file://` grammar overrides
   - `lsp.strz-lsp.binary.path` local binary overrides
   - optional `STRZ_LSP_BIN` terminal overrides

This preserves fast iteration while keeping published installs reproducible.

## Artifact matrix

| Artifact                           | Home                                           | Primary consumers                                                 | Distribution form            | Release unit                |
| ---------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------- | ---------------------------- | --------------------------- |
| Tree-sitter grammar + parser crate | this repo root                                 | Zed grammar loader, Rust parser consumers, `strz-analysis` | source repository            | git commit / pinned `rev`   |
| `strz-analysis`             | workspace crate in this repo                   | `strz-lsp`, `strz`                              | source-only workspace member | none initially              |
| `strz-lsp`                  | workspace crate in this repo                   | `strz`                                                 | source-only workspace member | none directly               |
| `strz`                             | `strz` workspace crate in this repo | Zed extension, other editors, manual users                        | platform binaries / archives | GitHub release tag + assets |
| `zed-structurizr`                  | separate repo                                  | Zed users                                                         | Zed extension package        | extension version           |

## Packaging decisions

The first rollout should make these decisions explicit.

## 1. Do not bundle the server in the Zed extension repo

Published Zed extensions are expected to locate or download language servers rather than ship the server binary as part of the extension source itself.

So:

- do not commit `strz` binaries into `zed-structurizr`
- do not make extension publication depend on checking binaries into that repo

The extension should remain a launcher/packager, not a binary-storage repository.

## 2. Do not require end users to install the server manually

Manual installation via `PATH` is a useful fallback and a good early dogfooding path.

But it should not be the primary stable-user story.

Recommended stable story:

- the extension downloads a pinned GitHub release asset when no explicit override or `PATH` binary is available

This matches the Zed extension model better than telling every user to run `cargo install` or maintain their own binary manually.

## 3. Do not publish the analysis crate as part of the first editor rollout

The analysis crate is an internal architecture boundary, not a user-facing distribution artifact.

That means:

- it should be built from source as a workspace member
- it does not need its own packaging channel initially
- it should not delay the editor rollout by introducing crates.io or separate binary-distribution concerns too early

If a later non-LSP consumer emerges, that can be revisited intentionally.

## 4. Do not rely on “latest release” for stable installs

Published extension releases should use a pinned LSP release tag.

Why:

- reproducibility
- testability
- rollback friendliness
- clearer bug reports

So the extension should not treat “latest GitHub release” as its default stable behavior.

## 5. Do not package grammar binaries separately

Zed already consumes the grammar from this repository by git revision.

That means:

- the grammar does not need a separate binary packaging story for the Zed integration
- the extension's grammar update surface remains a pinned source revision

This keeps the grammar aligned with Zed’s existing loader model.

## LSP acquisition modes

The LSP should be acquirable in three modes, ordered from most local to most published.

## Mode A: explicit local binary override

Use:

- `lsp.strz-lsp.binary.path = "/absolute/path/to/strz"`
- or, for one-shot terminal launches, `STRZ_LSP_BIN=/absolute/path/to/strz`

This is the preferred contributor loop because it:

- avoids editing extension code for local experiments
- keeps binary choice explicit
- works even when the extension has download logic
- maps cleanly to Zed's native binary override model

This mode is for:

- active development
- debugging local changes
- testing unpublished binaries

## Mode B: user-managed binary on `PATH`

Use:

- `worktree.which("strz")`

This is the simplest fallback for:

- contributors who already build/install the binary
- advanced users
- early pre-release testing before download automation exists

This is useful, but it should remain a fallback rather than the only supported stable-user path.

## Mode C: pinned GitHub release asset

Use:

- a release tag pinned in extension code
- a platform-specific archive from this repository’s GitHub releases

This is the primary stable-user packaging path.

It should be the default when:

- no explicit override exists
- no suitable `PATH` binary exists

This gives the extension a reproducible editor-friendly install story without requiring a third packaging system.

## Release asset shape

The first release assets for `strz` should stay boring and deterministic.

## Archive naming

Recommended naming pattern:

- `strz-macos-aarch64.tar.gz`
- `strz-macos-x86_64.tar.gz`
- `strz-linux-x86_64.tar.gz`
- `strz-windows-x86_64.zip`

The exact supported matrix can grow later, but the naming pattern should stay stable.

## Archive contents

Each archive should extract to a predictable top-level executable path:

- `strz`
- or `strz.exe` on Windows

The extension should then launch that executable with:

- `server`

Avoid nested release-directory layouts like:

- `strz-v0.4.0-macos-aarch64/strz`

unless the extension code genuinely needs them.

A flat extracted path is much easier for the extension launcher to manage.

## Recommended asset source

Assets should be published from this repository, alongside the LSP source.

Why:

- keeps provenance simple
- avoids inventing another distribution repo
- keeps grammar/LSP release notes together
- matches the topology decision that grammar + analysis + LSP live here

## Versioning and pinning model

The project should treat grammar and LSP version references as related but separate pins.

## Grammar pin

The Zed extension should continue to pin:

- a grammar git revision in `extensions.toml`

That pin controls:

- grammar parsing behavior
- extension-side syntax query compatibility

## LSP pin

The Zed extension should separately pin:

- an LSP release tag in extension Rust code

That pin controls:

- the downloaded server binary
- protocol behavior
- semantic diagnostics/navigation behavior

## Why separate pins matter

Even though grammar and LSP live in the same repository, they should not be treated as one forced release unit.

That lets the project ship:

- grammar-only updates
- LSP-only updates
- or matched updates

without forcing unnecessary extension churn.

## Contributor loops

The dev loop should stay different depending on what is being changed.

## Loop 1: grammar-only work today

This is the current reality of this repository.

Use the existing Justfile commands:

```sh
just generate
just test-rust-fast
just test-grammar
```

Use:

```sh
just test
```

when you want the broader Rust + grammar validation pass before landing or releasing grammar changes.

For local Zed iteration:

- install `/Users/rob/dev/zed-structurizr` as a dev extension
- temporarily point its grammar entry at `file:///Users/rob/dev/tree-sitter-structurizr` with `path = "crates/strz-grammar"`
- open a representative `.dsl` file such as `big-bank.dsl`

This loop does not require an LSP yet.

## Loop 2: LSP implementation work

Now that [`crates/strz-analysis/`](../../../crates/strz-analysis/), [`crates/strz-lsp/`](../../../crates/strz-lsp/), and [`crates/strz/`](../../../crates/strz/) exist, the day-to-day loop should stay mostly in this repository.

Recommended loop:

1. update grammar, analysis, or LSP code here
1. run grammar validation:
   - `just test-grammar`
1. run targeted Rust validation:
   - parser crate tests as already needed
   - analysis crate tests
   - LSP crate tests
1. build the local `strz` binary
1. point Zed at the local binary only when integration behavior needs checking

The important workflow principle is:

- most LSP work should not require immediately publishing extension changes

That is one of the main benefits of keeping grammar + analysis + LSP together in this repository.

## Loop 3: full integration work across both repos

When a change spans:

- grammar behavior
- LSP behavior
- and Zed extension launch/package behavior

use the integrated dev loop:

1. build the local `strz` binary in this repo
1. set `lsp.strz-lsp.binary.path` to that binary
1. point the Zed dev extension at the local grammar repo with `file://`
1. run Zed in foreground mode
1. smoke-test both editor-native and LSP-provided behavior

For one-off terminal launches, step 2 can temporarily use
`strz_lsp_BIN=... zed --foreground` instead, but only when starting a
fresh Zed instance. If Zed is already running, prefer
`lsp.strz-lsp.binary.path`.

This is the slowest loop, so it should be reserved for:

- cross-repo integration checks
- launcher/download behavior
- end-to-end smoke testing

It should not be the default loop for ordinary grammar or server work.

## Loop 4: release rehearsal

Before updating the published extension, test the exact packaging story that users will see.

Recommended rehearsal:

1. create or identify the intended `strz` release assets
1. ensure the extension is pinned to the intended LSP tag
1. remove `lsp.strz-lsp.binary.path` and `strz_lsp_BIN` overrides
1. ensure the result works via:
   - downloaded asset path
   - or, when explicitly testing it, `PATH` fallback
1. smoke-test representative `.dsl` files

This catches packaging mistakes that local override workflows can hide.

## Release scenarios

The project should recognize three normal release shapes.

## Scenario A: grammar-only update

Examples:

- grammar correctness fix
- query update
- syntax-highlighting/folding alignment

Recommended flow:

1. land grammar change here
1. validate with existing grammar/parser tests
1. update the Zed extension's pinned grammar `rev` only if the extension should consume the new grammar immediately
1. keep the LSP tag unchanged unless the server actually needs rebuilding or republishing

This should be the cheapest release shape.

## Scenario B: LSP-only update

Examples:

- navigation fix
- diagnostic fix
- workspace-index behavior change
- extension launch logic unchanged

Recommended flow:

1. land LSP/analysis changes here
1. build and validate the LSP
1. cut a new LSP release tag and platform assets
1. update the Zed extension's pinned LSP tag
1. keep the grammar `rev` unchanged unless the server depends on newer grammar behavior

This is the main reason grammar and LSP pins should stay separate.

## Scenario C: matched grammar + LSP update

Examples:

- grammar change unlocks a new semantic extraction surface
- query/export changes affect the server implementation
- syntax and semantic fixes are intended to ship together

Recommended flow:

1. land both changes here
1. validate grammar and LSP behavior together
1. cut or select the intended grammar commit and LSP release assets
1. update both pins in the Zed extension
1. smoke-test as one integrated release

This is the heaviest but still normal shape.

## Smoke-test gate

Before treating a packaging/dev-loop decision as “done”, the project should be able to exercise:

- a grammar-only edit path
- a local LSP binary override path
- a published-like downloaded binary path

And validate:

- grammar still loads in Zed
- extension-owned queries still behave sensibly
- the server starts and answers bounded MVP requests
- include diagnostics and cross-file navigation remain stable

Use:

- `/Users/rob/dev/zed-structurizr/big-bank.dsl`
- [`crates/strz-lsp/tests/fixtures/includes/`](../../../crates/strz-lsp/tests/fixtures/includes/)
- [`crates/strz-lsp/tests/fixtures/identifiers/`](../../../crates/strz-lsp/tests/fixtures/identifiers/)
- [`crates/strz-lsp/tests/fixtures/relationships/`](../../../crates/strz-lsp/tests/fixtures/relationships/)

as the first representative smoke-test set.

## Recommended automation direction

The first packaging story should stay simple, but it should point toward eventual automation.

## This repository

Recommended eventual automation:

- continue running grammar validation with the existing Justfile commands
- add release automation for `strz` platform builds from the `strz` crate
- publish release assets from tagged commits

The key design choice is:

- GitHub releases are the first packaging channel

There is no need to invent:

- npm packaging
- Homebrew formulas
- separate installer repositories
- multi-channel updater logic

before the basic binary-release flow works.

## Zed extension repo

Recommended eventual automation:

- keep extension versioning independent
- update the pinned grammar `rev` and pinned LSP tag deliberately
- smoke-test before publishing extension updates

This should remain a downstream packaging step, not something that the grammar/LSP repo tries to hide.

## What this note intentionally does not decide

This note does **not** freeze:

- exact GitHub Actions workflow files
- exact semantic-versioning rules
- whether crates should later be published to crates.io
- whether another editor eventually wants a different acquisition strategy
- whether preview/nightly server channels are worthwhile

Those can come later if real release pressure appears.

The important thing now is the baseline operational shape:

- source here
- released server binaries here
- extension package there
- local overrides for contributors
- pinned references for published installs

## What this unblocks

Once this note is followed:

- contributors know which loop to use for grammar-only vs LSP vs integration work
- the project has a concrete answer to “how should Zed find the server?”
- releases can evolve without forcing every change through one monolithic update path
- the LSP can be editor-agnostic at the binary level while still integrating cleanly with Zed

That keeps the project practical for day-to-day work as the LSP starts becoming real.

## Sources

- [`docs/lsp/01-foundations/repository-topology.md`](../01-foundations/repository-topology.md)
- [`docs/lsp/02-design/lsp-crate-skeleton.md`](../02-design/lsp-crate-skeleton.md)
- [`docs/lsp/03-delivery/zed-extension-language-server-wiring.md`](zed-extension-language-server-wiring.md)
- [`docs/lsp/01-foundations/overview.md`](../01-foundations/overview.md)
- `/Users/rob/dev/tree-sitter-structurizr/Justfile`
- `/Users/rob/dev/zed-structurizr/README.md`
- `https://zed.dev/docs/extensions/languages`
- `https://zed.dev/docs/extensions/developing-extensions`
- `https://docs.rs/zed_extension_api/latest/zed_extension_api/trait.Extension.html`
