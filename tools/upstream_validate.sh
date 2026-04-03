#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: tools/upstream_validate.sh <workspace-path> [<workspace-path> ...]

Run structurizr/structurizr validate against explicit workspace entrypoints.
Paths are resolved relative to the repository root when they are not absolute.
Directory inputs prefer workspace.dsl or workspace.json when present; otherwise
they must contain exactly one top-level .dsl or .json file.
EOF
}

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "${script_dir}/.." && pwd -P)"

case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
esac

if [[ $# -eq 0 ]]; then
    usage >&2
    exit 2
fi

if ! command -v docker >/dev/null 2>&1; then
    printf 'docker is required to run upstream validation\n' >&2
    exit 1
fi

resolve_input_path() {
    local input_path="$1"
    local absolute_path
    local resolved_dir
    local resolved_base
    local candidate_path
    local -a candidate_paths=()
    local nullglob_was_set=0

    if [[ "${input_path}" == /* ]]; then
        absolute_path="${input_path}"
    else
        absolute_path="${repo_root}/${input_path}"
    fi

    if [[ ! -e "${absolute_path}" ]]; then
        printf 'workspace path does not exist: %s\n' "${input_path}" >&2
        return 1
    fi

    if [[ -d "${absolute_path}" ]]; then
        for candidate_path in workspace.dsl workspace.json; do
            if [[ -f "${absolute_path}/${candidate_path}" ]]; then
                (
                    cd -- "${absolute_path}" &&
                        printf '%s/%s\n' "$(pwd -P)" "${candidate_path}"
                )
                return
            fi
        done

        if shopt -q nullglob; then
            nullglob_was_set=1
        fi
        shopt -s nullglob
        candidate_paths=("${absolute_path}"/*.dsl "${absolute_path}"/*.json)
        if [[ "${nullglob_was_set}" -eq 0 ]]; then
            shopt -u nullglob
        fi

        if [[ "${#candidate_paths[@]}" -eq 1 ]]; then
            candidate_path="$(basename -- "${candidate_paths[0]}")"
            (
                cd -- "${absolute_path}" &&
                    printf '%s/%s\n' "$(pwd -P)" "${candidate_path}"
            )
            return
        fi

        if [[ "${#candidate_paths[@]}" -eq 0 ]]; then
            printf 'workspace directory does not contain a top-level .dsl or .json file: %s\n' "${input_path}" >&2
            return 1
        fi

        printf 'workspace directory is ambiguous; pass an explicit .dsl or .json file path: %s\n' "${input_path}" >&2
        return 1
    fi

    if [[ ! -f "${absolute_path}" ]]; then
        printf 'workspace path is not a regular file: %s\n' "${input_path}" >&2
        return 1
    fi

    if [[ "${absolute_path##*/}" != *.dsl && "${absolute_path##*/}" != *.json ]]; then
        printf 'workspace path must point to a .dsl or .json file: %s\n' "${input_path}" >&2
        return 1
    fi

    resolved_dir="$(dirname -- "${absolute_path}")"
    resolved_base="$(basename -- "${absolute_path}")"
    (cd -- "${resolved_dir}" && printf '%s/%s\n' "$(pwd -P)" "${resolved_base}")
}

container_path_for() {
    local absolute_path="$1"
    local repo_prefix="${repo_root}/"
    local relative_path

    case "${absolute_path}" in
        "${repo_root}")
            printf '/workspace'
            ;;
        "${repo_root}"/*)
            relative_path="${absolute_path#"$repo_prefix"}"
            printf '/workspace/%s' "${relative_path}"
            ;;
        *)
            printf 'workspace path must live inside the repository: %s\n' "${absolute_path}" >&2
            return 1
            ;;
    esac
}

failed=0
total=$#
index=1

for workspace_path in "$@"; do
    if ! absolute_path="$(resolve_input_path "${workspace_path}")"; then
        printf '[%d/%d] failed %s\n' "${index}" "${total}" "${workspace_path}" >&2
        failed=1
        index=$((index + 1))
        continue
    fi

    if ! container_path="$(container_path_for "${absolute_path}")"; then
        printf '[%d/%d] failed %s\n' "${index}" "${total}" "${workspace_path}" >&2
        failed=1
        index=$((index + 1))
        continue
    fi

    printf '[%d/%d] validating %s\n' "${index}" "${total}" "${workspace_path}"
    if docker run --rm \
        -v "${repo_root}:/workspace:ro" \
        -w /workspace \
        structurizr/structurizr \
        validate \
        -workspace "${container_path}"; then
        printf '[%d/%d] ok %s\n' "${index}" "${total}" "${workspace_path}"
    else
        printf '[%d/%d] failed %s\n' "${index}" "${total}" "${workspace_path}" >&2
        failed=1
    fi

    index=$((index + 1))
done

exit "${failed}"
