## Issue

The repository lacks a reusable helper for running its own workspace entrypoints
through upstream `structurizr/structurizr validate`.

The benchmark-mega follow-up found several real compatibility problems
after manually running `docker run --rm structurizr/structurizr validate ...`,
and there is no checked-in helper that contributors can reuse.

## Root Cause

[`tools/upstream_audit.rs`](../tools/upstream_audit.rs) and
[`Justfile`](../Justfile) already give us `just audit-upstream` for
grammar-facing parity checks against upstream-owned sample files, but that flow
does not validate this repository's own DSL workspaces with the upstream CLI.

[`crates/structurizr-cli/src/check.rs`](../crates/structurizr-cli/src/check.rs)
already surfaces local semantic diagnostics, but it enforce
upstream-only validation rules by itself.

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
command surface, and complements the existing grammar audit with an opt-in
workspace-parity gate.
