#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: tools/bench_black_box.sh [--mode quick|stable] [--output-dir DIR] [--binary PATH] [--dry-run]

Run release-binary black-box benchmarks for the contributor CLI and the stdio
LSP entrypoint. Results are exported as Hyperfine JSON files.
EOF
}

shell_join() {
    local joined=""
    local quoted_part

    for quoted_part in "$@"; do
        local escaped
        printf -v escaped '%q' "$quoted_part"
        if [[ -n "$joined" ]]; then
            joined+=" "
        fi
        joined+="$escaped"
    done

    printf '%s' "$joined"
}

build_command() {
    local prefix="${STRZ_BENCH_CMD_PREFIX:-}"
    local command

    command="$(shell_join "$@")"
    if [[ -n "$prefix" ]]; then
        printf '%s %s' "$prefix" "$command"
    else
        printf '%s' "$command"
    fi
}

build_lsp_replay_command() {
    if command -v uv >/dev/null 2>&1; then
        build_command \
            uv run --python 3.12 "${REPO_ROOT}/tools/lsp_replay.py" \
            --server "${BINARY}" \
            --case "$1"
    else
        build_command \
            python3 "${REPO_ROOT}/tools/lsp_replay.py" \
            --server "${BINARY}" \
            --case "$1"
    fi
}

run_hyperfine() {
    local output_file="$1"
    shift
    local -a commands=("$@")

    if [[ "${DRY_RUN}" == "true" ]]; then
        printf 'hyperfine --warmup %q --runs %q --export-json %q' \
            "${WARMUP}" "${RUNS}" "${output_file}"
        printf ' %q' "${commands[@]}"
        printf '\n'
        return
    fi

    hyperfine \
        --warmup "${WARMUP}" \
        --runs "${RUNS}" \
        --export-json "${output_file}" \
        "${commands[@]}"
}

MODE="quick"
OUTPUT_DIR=""
BINARY="target/release/strz"
DRY_RUN="false"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --binary)
            BINARY="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN="true"
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
    quick)
        WARMUP=1
        RUNS=5
        ;;
    stable)
        WARMUP=2
        RUNS=10
        ;;
    *)
        printf 'unsupported benchmark mode: %s\n' "${MODE}" >&2
        exit 2
        ;;
esac

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -z "${OUTPUT_DIR}" ]]; then
    OUTPUT_DIR="${REPO_ROOT}/tmp/benchmark-results/${MODE}"
fi
if [[ "${BINARY}" != /* ]]; then
    BINARY="${REPO_ROOT}/${BINARY}"
fi

mkdir -p "${OUTPUT_DIR}"

if [[ "${DRY_RUN}" != "true" ]]; then
    if ! command -v hyperfine >/dev/null 2>&1; then
        printf 'hyperfine is required for black-box benchmarks\n' >&2
        exit 1
    fi
    if ! command -v uv >/dev/null 2>&1 && ! command -v python3 >/dev/null 2>&1; then
        printf 'uv or python3 is required for LSP replay benchmarks\n' >&2
        exit 1
    fi
    if [[ ! -x "${BINARY}" ]]; then
        printf 'benchmark binary does not exist or is not executable: %s\n' "${BINARY}" >&2
        exit 1
    fi
fi

run_hyperfine \
    "${OUTPUT_DIR}/check.json" \
    "$(build_command "${BINARY}" check "${REPO_ROOT}/tests/lsp/workspaces/directory-include")" \
    "$(build_command "${BINARY}" check "${REPO_ROOT}/tests/lsp/workspaces/big-bank-plc")" \
    "$(build_command "${BINARY}" check "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega/mega.dsl")" \
    "$(build_command "${BINARY}" check \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-00/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-01/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-02/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-03/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-04/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-05/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-06/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-07/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-08/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-09/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-10/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-11/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-13/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-14/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-15/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-16/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-17/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-18/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-19/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-20/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-21/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-22/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-23/workspace.dsl")"

run_hyperfine \
    "${OUTPUT_DIR}/dump-workspace.json" \
    "$(build_command "${BINARY}" dump workspace "${REPO_ROOT}/tests/lsp/workspaces/directory-include")" \
    "$(build_command "${BINARY}" dump workspace "${REPO_ROOT}/tests/lsp/workspaces/big-bank-plc")" \
    "$(build_command "${BINARY}" dump workspace "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega/mega.dsl")" \
    "$(build_command "${BINARY}" dump workspace \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-00/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-01/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-02/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-03/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-04/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-05/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-06/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-07/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-08/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-09/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-10/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-11/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-13/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-14/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-15/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-16/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-17/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-18/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-19/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-20/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-21/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-22/workspace.dsl" \
        "${REPO_ROOT}/tests/lsp/workspaces/benchmark-mega-multi-root/ws-23/workspace.dsl")"

run_hyperfine \
    "${OUTPUT_DIR}/lsp-session.json" \
    "$(build_lsp_replay_command small)" \
    "$(build_lsp_replay_command large)" \
    "$(build_lsp_replay_command mega)" \
    "$(build_lsp_replay_command mega-multi-root)"

printf 'black-box benchmark results written to %s\n' "${OUTPUT_DIR}"
