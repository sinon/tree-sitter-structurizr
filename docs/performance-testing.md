# Performance testing

This repository has three complementary performance loops:

- in-process Rust benchmarks for the analysis and LSP crates
- black-box release-binary benchmarks for the contributor CLI and stdio server
- CodSpeed-compatible runs for the benchmark surface that CI tracks on pull requests

Use the lightest loop that can answer the question you have, then move outward only when you need parity with CI or a user-visible command path.

## Tooling

Baseline local tooling:

- `cargo`
- `just`
- `cargo-codspeed`
- `hyperfine`
- `uv`

Useful local profilers:

- `samply`
- macOS `sample`
- macOS `xctrace`

Keep observability env vars such as `RUST_LOG`, `STRZ_LOG_FORMAT`, `STRZ_LOG_FILE`, and `STRZ_TEST_LOG` unset for normal performance runs unless you are explicitly measuring logging overhead.

## Benchmark surface

The checked-in benchmark matrix is intentionally small and stable:

- `analysis/document`
  - `small_direct_references`
  - `medium_people_and_software_systems`
  - `large_big_bank_workspace`
- `analysis/workspace`
  - `small_minimal_scan`
  - `medium_directory_include`
  - `large_big_bank_plc`
- `lsp/session`
  - `small_named_relationship_definition`
  - `large_big_bank_document_symbols`

The black-box suite adds release-binary checks for:

- `strz check`
- `strz dump workspace`
- replayed `strz server` sessions via [`tools/lsp_replay.py`](../tools/lsp_replay.py)

## Quick start

From the repository root:

```sh
just bench-rust
just bench-black-box
just bench-perf
just bench-perf-stable
```

Choose the loop based on the question:

- use `just bench-rust` for the fastest in-process analysis/LSP iteration
- use `just bench-black-box` for user-visible release-binary timing
- use `just bench-perf` when you want both plus captured environment metadata
- use `just bench-perf-stable` when you want the same combined flow with the most reproducible local setup the repo currently offers

[`tools/run_benchmarks.sh`](../tools/run_benchmarks.sh) writes environment capture and benchmark output under `tmp/benchmark-results/` by default, so comparison artifacts stay project-local.

## Targeted in-process benches

Use targeted `cargo bench` commands when you already know which benchmark case you want to isolate:

```sh
cargo bench -p structurizr-analysis --bench analysis small_minimal_scan -- --noplot
cargo bench -p structurizr-analysis --bench analysis large_big_bank_workspace -- --noplot
cargo bench -p structurizr-lsp --bench session small_named_relationship_definition -- --noplot
cargo bench -p structurizr-lsp --bench session large_big_bank_document_symbols -- --noplot
```

This is the best loop for verifying whether a code change affects one benchmarked path without rerunning unrelated release-binary work.

## Black-box release-binary benches

Build the contributor CLI once, then run the Hyperfine-backed suite:

```sh
cargo build -p structurizr-cli --bin strz --release
tools/bench_black_box.sh --mode quick --output-dir tmp/benchmark-results/quick --binary target/release/strz
```

The script writes JSON outputs for:

- `check.json`
- `dump-workspace.json`
- `lsp-session.json`

Use this loop when you care about the whole command surface rather than only crate-local inner loops.

## CodSpeed parity

The CI performance workflow builds CodSpeed benches package-by-package:

```sh
cargo codspeed build -p structurizr-analysis -p structurizr-lsp
cargo codspeed run -p structurizr-analysis
cargo codspeed run -p structurizr-lsp
```

Local CodSpeed runs are still useful even outside a supported measurement environment because they verify:

- the expected benchmark suites still build
- the benchmark set matches CI
- a suspected regression reproduces on the same harness shape

Outside the CodSpeed-supported environment you should expect the harness to report that it ran successfully without publishing comparable dashboard metrics.

## Profiling hot loops

Start with a targeted benchmark command, then wrap it in a profiler.

### `samply`

For a focused analysis benchmark:

```sh
samply record --save-only --no-open -o tmp/analysis-small.json.gz -- \
  cargo bench -p structurizr-analysis --bench analysis small_minimal_scan -- --noplot
```

For an LSP session benchmark:

```sh
samply record --save-only --no-open -o tmp/lsp-large-session.json.gz -- \
  cargo bench -p structurizr-lsp --bench session large_big_bank_document_symbols -- --noplot
```

These saved artifacts stay in `tmp/` so they are easy to compare or discard after the investigation.

### Direct LSP replay

If you want to profile the release-binary LSP path outside Criterion:

```sh
uv run --python 3.12 tools/lsp_replay.py --server target/release/strz --case large
```

This follows the same small/large session shapes the black-box benchmark script uses.

## Stable comparisons

When you want lower-noise local comparisons:

- run on an otherwise quiet machine
- compare one change at a time
- prefer targeted benchmarks before the full suite
- keep logging disabled
- use `just bench-perf-stable` when you want the repo's combined comparison flow

On Linux, set `STRZ_BENCH_CPUSET` before the stable run to request CPU pinning through `taskset`:

```sh
STRZ_BENCH_CPUSET=2 just bench-perf-stable
```

On macOS the same command still captures environment metadata, but there is no equivalent automatic CPU pinning in the checked-in scripts.

## Suggested investigation workflow

1. Run `just bench-rust` to see which benchmark family moved.
2. Narrow to one targeted `cargo bench` case.
3. If the change affects CLI or server UX, run `just bench-black-box` or the direct `lsp_replay.py` command.
4. If the regression needs CI-parity confirmation, run the CodSpeed-compatible build/run pair.
5. Capture one profiler artifact for the narrowed case before making broader structural changes.
6. After the optimization, rerun the same targeted benchmark first, then the broader suite you used as the baseline.
