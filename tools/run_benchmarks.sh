#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: tools/run_benchmarks.sh [--mode quick|stable] [--results-dir DIR] [--skip-rust-benches] [--skip-black-box]

Run the repository's performance suite with a small amount of environment
capture so local comparisons stay grounded in machine context.
EOF
}

MODE="quick"
RESULTS_DIR=""
SKIP_RUST_BENCHES="false"
SKIP_BLACK_BOX="false"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --results-dir)
            RESULTS_DIR="$2"
            shift 2
            ;;
        --skip-rust-benches)
            SKIP_RUST_BENCHES="true"
            shift
            ;;
        --skip-black-box)
            SKIP_BLACK_BOX="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
done

case "${MODE}" in
    quick|stable)
        ;;
    *)
        printf 'unsupported benchmark mode: %s\n' "${MODE}" >&2
        exit 2
        ;;
esac

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -z "${RESULTS_DIR}" ]]; then
    RESULTS_DIR="${REPO_ROOT}/tmp/benchmark-results/${MODE}"
fi
mkdir -p "${RESULTS_DIR}"

capture_environment() {
    {
        printf 'mode: %s\n' "${MODE}"
        printf 'system: %s %s %s\n' "$(uname -s)" "$(uname -r)" "$(uname -m)"
        rustc --version
        cargo --version
        if command -v uv >/dev/null 2>&1; then
            uv --version
        fi
        if [[ -n "${STRZ_BENCH_CPUSET:-}" ]]; then
            printf 'requested_cpuset: %s\n' "${STRZ_BENCH_CPUSET}"
        fi

        case "$(uname -s)" in
            Darwin)
                sysctl -n machdep.cpu.brand_string 2>/dev/null | sed 's/^/cpu: /'
                sysctl -n hw.logicalcpu 2>/dev/null | sed 's/^/logical_cpu_count: /'
                ;;
            Linux)
                if command -v lscpu >/dev/null 2>&1; then
                    lscpu
                fi
                ;;
        esac
    } > "${RESULTS_DIR}/environment.txt"
}

capture_environment

BENCH_PREFIX=()
export STRZ_BENCH_CMD_PREFIX=""

run_with_prefix() {
    if [[ ${#BENCH_PREFIX[@]} -gt 0 ]]; then
        "${BENCH_PREFIX[@]}" "$@"
    else
        "$@"
    fi
}

if [[ "${MODE}" == "stable" && -n "${STRZ_BENCH_CPUSET:-}" ]]; then
    if [[ "$(uname -s)" == "Linux" ]]; then
        if command -v taskset >/dev/null 2>&1; then
            BENCH_PREFIX=(taskset -c "${STRZ_BENCH_CPUSET}")
            export STRZ_BENCH_CMD_PREFIX="taskset -c ${STRZ_BENCH_CPUSET}"
        else
            printf 'warning: STRZ_BENCH_CPUSET was requested, but taskset is unavailable\n' >&2
        fi
    else
        printf 'note: STRZ_BENCH_CPUSET is only applied automatically on Linux\n' >&2
    fi
fi

if [[ "${SKIP_RUST_BENCHES}" != "true" ]]; then
    run_with_prefix cargo bench -p structurizr-analysis --bench analysis
    run_with_prefix cargo bench -p structurizr-lsp --bench session
fi

if [[ "${SKIP_BLACK_BOX}" != "true" ]]; then
    cargo build -p structurizr-cli --bin strz --release
    "${REPO_ROOT}/tools/bench_black_box.sh" \
        --mode "${MODE}" \
        --output-dir "${RESULTS_DIR}" \
        --binary "${REPO_ROOT}/target/release/strz"
fi

printf 'combined benchmark results written to %s\n' "${RESULTS_DIR}"
