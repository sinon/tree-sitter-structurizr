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
    cargo nextest run
    cargo test --doc

test-rust-fast:
    cargo nextest run

audit-upstream:
    cargo test --test upstream_audit -- --ignored --nocapture

audit-upstream-all:
    STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 cargo test --test upstream_audit -- --ignored --nocapture

test-grammar:
    tree-sitter test

check: generate test

playground:
    tree-sitter build --wasm
    tree-sitter playground
