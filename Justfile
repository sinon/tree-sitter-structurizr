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

test-grammar:
    tree-sitter test

check: generate test

playground:
    tree-sitter build --wasm
    tree-sitter playground
