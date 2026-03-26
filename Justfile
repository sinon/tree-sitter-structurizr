set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

generate:
    tree-sitter generate

build:
    cargo build

build-wasm:
    tree-sitter build --wasm

test: test-rust test-grammar

test-rust:
    cargo nextest run --no-fail-fast
    cargo test --doc

test-rust-fast:
    cargo nextest run

audit-upstream:
    cargo +nightly -Zscript tools/upstream_audit.rs

audit-upstream-all:
    STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 cargo +nightly -Zscript tools/upstream_audit.rs

test-grammar:
    tree-sitter test

check: generate test

playground:
    tree-sitter build --wasm
    tree-sitter playground
