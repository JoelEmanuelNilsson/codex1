#!/usr/bin/env bash
# Shared V2 codex1 binary resolver.
#
# Source this file (don't execute):
#
#   source "$(cd "$(dirname "$0")" && pwd)/lib/resolve-codex1.sh"
#   BIN="$(resolve_codex1 "${REPO_ROOT}")" || exit $?
#
# Resolution order:
#   1. $CODEX1_BIN (if set and passes is_v2_bin)
#   2. $REPO_ROOT/target/release/codex1 (if executable and V2)
#   3. $REPO_ROOT/target/debug/codex1   (if executable and V2)
#   4. `codex1` on PATH (if V2)
#   5. `codex1-v2` on PATH (legacy-development fallback, if V2)
#
# Exits the calling script (via `return 2`) with a diagnostic when no V2
# binary can be found. A V1 `codex1` on PATH is ignored — that's the whole
# point of the is_v2_bin probe.

# shellcheck shell=bash

is_v2_bin() {
  local bin="$1"
  [[ -z "${bin}" ]] && return 1
  local help_out
  if ! help_out="$("${bin}" --help 2>/dev/null)"; then
    return 1
  fi
  # V2 help lists BOTH of these top-level groups; V1 has neither.
  echo "${help_out}" | grep -q -E '(^| )mission-close( |$)' || return 1
  echo "${help_out}" | grep -q -E '(^| )parent-loop( |$)' || return 1
  return 0
}

resolve_codex1() {
  local repo_root="${1:-${PWD}}"

  if [[ -n "${CODEX1_BIN:-}" ]]; then
    if is_v2_bin "${CODEX1_BIN}"; then
      echo "${CODEX1_BIN}"
      return 0
    fi
    echo "resolve-codex1: \$CODEX1_BIN=${CODEX1_BIN} is not a V2 codex1 (lacks mission-close / parent-loop subcommands)" >&2
    return 2
  fi

  local candidate
  for candidate in \
      "${repo_root}/target/release/codex1" \
      "${repo_root}/target/debug/codex1"; do
    if [[ -x "${candidate}" ]] && is_v2_bin "${candidate}"; then
      echo "${candidate}"
      return 0
    fi
  done

  local path_codex1
  if path_codex1="$(command -v codex1 2>/dev/null)" \
      && [[ -n "${path_codex1}" ]] \
      && is_v2_bin "${path_codex1}"; then
    echo "${path_codex1}"
    return 0
  fi

  local path_codex1_v2
  if path_codex1_v2="$(command -v codex1-v2 2>/dev/null)" \
      && [[ -n "${path_codex1_v2}" ]] \
      && is_v2_bin "${path_codex1_v2}"; then
    echo "${path_codex1_v2}"
    return 0
  fi

  echo "resolve-codex1: no V2 codex1 binary found." >&2
  echo "  searched: \$CODEX1_BIN, ${repo_root}/target/{release,debug}/codex1, codex1 on PATH, codex1-v2 on PATH" >&2
  if [[ -n "${path_codex1}" ]]; then
    echo "  note: ${path_codex1} exists on PATH but is not a V2 binary (missing mission-close / parent-loop)" >&2
  fi
  echo "  fix: run 'cargo build -p codex1 --release' in this repo, or set CODEX1_BIN=/abs/path/to/v2/codex1" >&2
  return 2
}
