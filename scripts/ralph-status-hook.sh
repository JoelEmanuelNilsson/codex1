#!/usr/bin/env bash
# Codex1 V2 Ralph stop hook.
#
# Enforces the one contract Ralph owns: if any mission in <repo-root>/PLANS/
# has an active, unpaused parent loop, stop is blocked. Everything else is a
# pass-through.
#
# Usage (scan mode — the default Codex Stop hook path):
#   ralph-status-hook.sh [--repo-root <path>]
#   → scans <repo-root>/PLANS/*/STATE.json (default <repo-root> = $PWD).
#   → blocks stop when `codex1 status` says `stop_policy.allow_stop: false`
#     for any mission; allows stop otherwise.
#
# Usage (single-mission mode — explicit check for one id):
#   ralph-status-hook.sh <mission-id> [--repo-root <path>]
#
# Rationale: the Codex Stop hook command is a single static string, so it
# cannot thread the active mission id through. V2 refuses ambient mission
# *resolution* for CLI commands, but the Stop hook asking "does any mission
# in this repo want to block right now?" is a well-defined repo-state query
# that uses the same authoritative STATE.json that V2 writes. No env var
# plumbing, no separate pointer file, no fail-open on forgotten setup.
#
# Exit codes:
#   0 — stop allowed; Ralph is silent.
#   1 — stop blocked; reasons printed to stderr.
#   2 — hook itself failed (missing binary, unparseable JSON); treat as
#       blocking by default (fail-safe).
#
# Environment:
#   CODEX1_BIN   — override the resolved V2 binary.
#   PATH/cwd     — used by `resolve_codex1` and the default repo-root.

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/resolve-codex1.sh
source "${SCRIPT_DIR}/lib/resolve-codex1.sh"

MISSION=""
REPO_ROOT_ARG=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      REPO_ROOT_ARG="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '2,30p' "$0" >&2
      exit 0
      ;;
    -*|--*)
      echo "ralph-status-hook: unknown flag $1" >&2
      exit 2
      ;;
    *)
      if [[ -n "$MISSION" ]]; then
        echo "ralph-status-hook: too many positional args (got $MISSION, $1)" >&2
        exit 2
      fi
      MISSION="$1"
      shift
      ;;
  esac
done

# Resolve the mission repo-root.
#
# Precedence (first hit wins):
#   1. `--repo-root <path>` (explicit override; used by tests and debugging).
#   2. Nearest ancestor of $PWD that contains a `PLANS/` directory.
#   3. $PWD itself, if PLANS/ is right there.
#   4. $PWD as a bare default (produces "no missions" → allow stop).
#
# Step 2 is a bounded walk-up — we search for a specific marker (`PLANS/`),
# not an arbitrary config. Without this, the hook silently fails open when
# the runner's cwd is a subdirectory of the mission repo. V2's
# no-ambient-resolution rule applies to CLI commands (which must take
# `--mission <id>` explicitly), not to the hook's question of "which
# directory on disk contains this mission tree?"
find_mission_root() {
  local dir="${1:-$PWD}"
  while :; do
    if [[ -d "$dir/PLANS" ]]; then
      printf '%s' "$dir"
      return 0
    fi
    # Stop at filesystem root.
    local parent
    parent="$(dirname "$dir")"
    [[ "$parent" == "$dir" ]] && break
    dir="$parent"
  done
  # No PLANS/ anywhere above cwd — caller falls back to $PWD (will produce
  # "no missions to check, allow stop", which is the correct answer when
  # Codex is truly running outside any mission tree).
  return 1
}

if [[ -n "${REPO_ROOT_ARG}" ]]; then
  REPO_ROOT="${REPO_ROOT_ARG}"
else
  REPO_ROOT="$(find_mission_root "$PWD" || echo "$PWD")"
fi

# Resolve V2 binary. For resolver purposes we prefer the repo that owns
# *this* script (so the dev's own target/release/codex1 is found) rather
# than the caller's cwd (which might be a qualification tempdir).
BIN="$(resolve_codex1 "$(cd "${SCRIPT_DIR}/.." && pwd)")" || exit $?

# Parse one status envelope; emit "<allow>\t<reason>\t<display_message>".
parse_status_envelope() {
  local out="$1"
  python3 - "$out" <<'PY'
import json, sys
env = json.loads(sys.argv[1])
allow = "true" if env.get("stop_policy", {}).get("allow_stop") else "false"
reason = env.get("stop_policy", {}).get("reason", "")
msg = env.get("next_action", {}).get("display_message", "")
print(f"{allow}\t{reason}\t{msg}")
PY
}

# Run `codex1 status --mission <id> --repo-root <REPO_ROOT> --json`.
# Returns the stdout on success; exits 2 on invocation failure.
codex1_status() {
  local mid="$1"
  local out
  if ! out=$("${BIN}" --json --repo-root "${REPO_ROOT}" status --mission "${mid}" 2>/dev/null); then
    echo "ralph-status-hook: ${BIN} status failed for mission ${mid}" >&2
    exit 2
  fi
  printf '%s' "$out"
}

check_single_mission() {
  local mid="$1"
  local out allow reason msg
  out="$(codex1_status "$mid")"
  IFS=$'\t' read -r allow reason msg <<<"$(parse_status_envelope "$out")"
  if [[ "${allow}" == "true" ]]; then
    exit 0
  fi
  printf 'Ralph blocked stop: %s\n' "${reason:-active_parent_loop}" >&2
  [[ -n "${msg}" ]] && printf '%s\n' "${msg}" >&2
  exit 1
}

scan_all_missions() {
  local plans_dir="${REPO_ROOT}/PLANS"
  # No PLANS dir → no missions → allow stop.
  [[ -d "$plans_dir" ]] || exit 0

  local blocked=()
  # Use nullglob-style iteration — fall through silently when no matches.
  for mission_dir in "$plans_dir"/*/; do
    [[ -d "$mission_dir" ]] || continue
    [[ -f "${mission_dir}STATE.json" ]] || continue
    local mid
    mid="$(basename "$mission_dir")"

    local out allow reason msg
    # If a single mission's status blows up, skip rather than fail-open.
    # (An unreadable STATE.json with codex1 refusing it is itself suspicious;
    # we mark it as blocking to surface the problem.)
    if ! out=$("${BIN}" --json --repo-root "${REPO_ROOT}" status --mission "${mid}" 2>/dev/null); then
      blocked+=("${mid}: codex1 status failed (mission may be corrupt)")
      continue
    fi
    IFS=$'\t' read -r allow reason msg <<<"$(parse_status_envelope "$out")"
    if [[ "${allow}" == "false" ]]; then
      blocked+=("${mid}: ${reason:-active_parent_loop}${msg:+ — ${msg}}")
    fi
  done

  if [[ ${#blocked[@]} -eq 0 ]]; then
    exit 0
  fi

  printf 'Ralph blocked stop: active parent loop in %d mission(s)\n' "${#blocked[@]}" >&2
  local line
  for line in "${blocked[@]}"; do
    printf '  - %s\n' "$line" >&2
  done
  exit 1
}

if [[ -n "$MISSION" ]]; then
  check_single_mission "$MISSION"
else
  scan_all_missions
fi
