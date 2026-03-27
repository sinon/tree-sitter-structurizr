set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

generate:
    tree-sitter generate

build:
    cargo build

# The generated Rust build script currently emits constant string emptiness checks,
# so keep that one clippy lint disabled in the shared dev flow.
lint:
    cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery -A clippy::const_is_empty

build-wasm:
    tree-sitter build --wasm

test: test-rust test-grammar

test-analysis:
    cargo test -p structurizr-analysis

test-analysis-fast:
    cargo nextest run --workspace -p structurizr-analysis

test-rust:
    cargo nextest run --workspace --no-fail-fast
    cargo test --workspace --doc

test-rust-fast:
    cargo nextest run --workspace

audit-upstream:
    cargo +nightly -Zscript tools/upstream_audit.rs

audit-upstream-all:
    STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 cargo +nightly -Zscript tools/upstream_audit.rs

zizmor:
    zizmor --gh-token="$(gh auth token)" .github/ --persona pedantic

test-grammar:
    tree-sitter test

check: generate test
    @just lint

playground:
    tree-sitter build --wasm
    tree-sitter playground
