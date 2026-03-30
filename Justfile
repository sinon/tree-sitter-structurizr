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
lint-fix:
    cargo clippy --fix --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery -A clippy::const_is_empty

build-wasm:
    tree-sitter build --wasm

build-strz:
    cargo build -p structurizr-cli --bin strz --release

test: test-rust test-grammar

test-analysis:
    cargo test -p structurizr-analysis

test-analysis-fast:
    cargo nextest run --workspace -p structurizr-analysis

test-cli:
    cargo test -p structurizr-cli

bench-analysis:
    cargo bench -p structurizr-analysis --bench analysis

bench-lsp:
    cargo bench -p structurizr-lsp --bench session

bench-rust: bench-analysis bench-lsp

bench-black-box:
    cargo build -p structurizr-cli --bin strz --release
    tools/bench_black_box.sh --mode quick --output-dir tmp/benchmark-results/quick --binary target/release/strz

bench-black-box-stable:
    cargo build -p structurizr-cli --bin strz --release
    tools/bench_black_box.sh --mode stable --output-dir tmp/benchmark-results/stable --binary target/release/strz

bench-perf:
    tools/run_benchmarks.sh --mode quick

bench-perf-stable:
    tools/run_benchmarks.sh --mode stable

test-rust:
    cargo nextest run --workspace --no-fail-fast
    cargo test --workspace --doc

test-rust-fast:
    cargo nextest run --workspace

test-proptest *args:
    cargo test --workspace {{args}}

test-proptest-stress cases *args:
    PROPTEST_CASES="{{cases}}" cargo test --workspace {{args}}

rerun-proptest seed *args:
    PROPTEST_CASES=1 PROPTEST_RNG_SEED="{{seed}}" cargo test --workspace {{args}}

capture-proptest capture_dir *args:
    STRUCTURIZR_PROPTEST_CAPTURE_DIR="$PWD/{{capture_dir}}" cargo test --workspace {{args}}

rerun-and-capture-proptest seed capture_dir *args:
    PROPTEST_CASES=1 PROPTEST_RNG_SEED="{{seed}}" STRUCTURIZR_PROPTEST_CAPTURE_DIR="$PWD/{{capture_dir}}" cargo test --workspace {{args}}

audit-upstream:
    cargo +nightly -Zscript tools/upstream_audit.rs

audit-upstream-all:
    STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED=1 cargo +nightly -Zscript tools/upstream_audit.rs

zizmor:
    zizmor --gh-token="$(gh auth token)" .github/ --persona pedantic

zizmor-fix:
    zizmor --gh-token="$(gh auth token)" .github/ --persona pedantic --fix

test-grammar:
    tree-sitter test

fuzz-grammar iterations="10" edits="3":
    tree-sitter fuzz --iterations {{iterations}} --edits {{edits}}

fuzz-grammar-stress iterations="100" edits="5":
    tree-sitter fuzz --iterations {{iterations}} --edits {{edits}}

check: generate test
    @just lint

run-strz *args:
    cargo run -p structurizr-cli --bin strz -- {{args}}

playground:
    tree-sitter build --wasm
    tree-sitter playground
