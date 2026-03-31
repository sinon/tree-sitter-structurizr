## Issue

We now have local benchmark corpora and other checked-in DSL workspaces whose
acceptance by `strz check` is not enough to prove upstream Structurizr parity.
The benchmark-mega follow-up only found several real compatibility problems
after manually running `docker run --rm structurizr/structurizr validate ...`,
and there is no checked-in helper that contributors can reuse.

## Root Cause

[`tools/upstream_audit.rs`](../tools/upstream_audit.rs) compares our grammar
against upstream-owned sample files, but it does not validate this repository's
own DSL workspaces with the upstream CLI.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
validates only through the in-repo parser and workspace loader. That is useful,
but it cannot catch upstream-only semantic rules.

A naive `find . -name '*.dsl'` pass would also be wrong because much of the
repo contains valid fragments such as standalone `model { ... }` and
`views { ... }` files rather than top-level workspace entrypoints.

## Options

- Keep using ad-hoc manual Docker commands whenever a contributor wants
  upstream confirmation.
- Add a small repo-local script plus a `just` recipe that runs the upstream
  Docker validator against explicit workspace entrypoints or a checked-in
  allowlist.
- Build upstream validation directly into `strz` as a new subcommand.

## Proposed Option

Add a small tool under [`tools/`](../tools/) and a matching `just` recipe for
upstream parity checks. Keep it entrypoint-oriented rather than "all `.dsl`
files": accept explicit workspace roots and/or a checked-in manifest of known
standalone workspace files, mount the repo into the `structurizr/structurizr`
container, run `validate -workspace ...` for each entrypoint, and summarize
pass/fail in a CI-friendly way.

That keeps the integration lightweight, reuses the already-proven Docker
command surface, and gives future parser or fixture changes an easy opt-in
parity gate.
