#!/usr/bin/env bash
# Codex1 V2 Ralph stop hook.
#
# Round 13 P1 — parent-lane gate. The hook now reads the hook JSON
# from stdin (Claude Code always pipes it) and consults the
# `.codex1/parent-session.json` lease written by the SessionStart
# hook. If the current session_id does NOT own the lease, this is a
# secondary or standalone Claude session (a separate terminal, a
# reviewer subagent, etc.) and the hook exits 0 immediately without
# scanning. Only the parent-orchestrator session whose SessionStart
# claimed the lease runs the block-if-active scan.
#
# This closes the Round 13 P1 complaint that "reviewer/worker subagent
# stopping from the same repo can be blocked by the parent loop."
# Task/Agent-tool subagents fire SubagentStop (not registered), but
# separately-launched `claude` sessions share this Stop hook via the
# shared hooks.json; the lease is the only authority signal available.
#
# If no lease exists (SessionStart never fired, or released by
# SessionEnd), the hook exits 0. That means a deployment that
# doesn't wire SessionStart will never block — a deliberate
# fail-open, because "no claim = no parent" is safer than "every
# session blocks."
#
# Usage (scan mode — the default Codex Stop hook path):
#   ralph-status-hook.sh [--repo-root <path>]
#   → reads hook JSON on stdin (session_id used for lease check)
#   → scans <repo-root>/PLANS/*/STATE.json (default <repo-root> = $PWD)
#   → blocks stop when `codex1 status` says `stop_policy.allow_stop: false`
#     for any mission; allows stop otherwise.
#
# Usage (single-mission mode — explicit check for one id):
#   ralph-status-hook.sh <mission-id> [--repo-root <path>]
#
# Exit codes:
#   0 — stop allowed; Ralph is silent.
#   1 — stop blocked; reasons printed to stderr.
#   2 — hook itself failed (missing binary, unparseable JSON); treat as
#       blocking by default (fail-safe).
#
# Environment:
#   CODEX1_BIN          — override the resolved V2 binary.
#   CODEX1_SKIP_LANE_CHECK=1 — bypass the lease gate (test fixtures).
#   PATH/cwd            — used by `resolve_codex1` and the default repo-root.

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/resolve-codex1.sh
source "${SCRIPT_DIR}/lib/resolve-codex1.sh"

# Round 13 P1: drain stdin once so the lease check and the rest of the
# hook share the same session_id without either consuming the other's
# JSON. Claude Code v2.1+ guarantees session_id in every hook payload.
HOOK_INPUT=""
if [[ ! -t 0 ]]; then
  HOOK_INPUT="$(cat)"
fi
HOOK_SESSION_ID=""
if [[ -n "${HOOK_INPUT}" ]]; then
  HOOK_SESSION_ID="$(printf '%s' "${HOOK_INPUT}" | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    sid = data.get("session_id", "")
    if isinstance(sid, str):
        print(sid)
except Exception:
    pass
' 2>/dev/null || true)"
fi

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

# Round 13 P1 / Round 14 P1 parent-lane gate. A Stop event fires in
# every root Claude session in this repo. The lease
# (.codex1/parent-session.json) holds the session_id of the
# orchestrator that should enforce the block.
#
# Semantics:
#   - No lease                          → exit 0 (no parent to enforce).
#   - Lease + matching session_id       → scan (I am the parent).
#   - Lease + different session_id      → exit 0 (I am a secondary).
#   - Lease + no session_id on stdin    → scan (fail-closed; Round 14
#       P1: Ralph must not silently become a no-op when the runtime
#       doesn't pipe session_id — Codex Desktop, stripped hook input,
#       etc.).
#
# Bypassable via CODEX1_SKIP_LANE_CHECK=1 for test fixtures that cover
# the scan logic directly without hook plumbing.
if [[ "${CODEX1_SKIP_LANE_CHECK:-0}" != "1" ]]; then
  LEASE_FILE="${REPO_ROOT}/.codex1/parent-session.json"
  if [[ ! -f "${LEASE_FILE}" ]]; then
    exit 0
  fi
  LEASE_SID="$(python3 -c '
import json, sys
try:
    with open(sys.argv[1]) as f:
        print(json.load(f).get("session_id", ""))
except Exception:
    pass
' "${LEASE_FILE}" 2>/dev/null || true)"
  # Only exit 0 when we can positively identify ourselves as a
  # different session. Unknown identity (empty HOOK_SESSION_ID) falls
  # through to the scan — the lease's existence proves a parent is
  # live, and blocking a Stop we can't identify is safer than
  # silently disabling Ralph.
  if [[ -n "${HOOK_SESSION_ID}" && "${LEASE_SID}" != "${HOOK_SESSION_ID}" ]]; then
    exit 0
  fi
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
    local mid
    mid="$(basename "$mission_dir")"

    # Round 7 P2: a mission-shaped directory without STATE.json is
    # corrupt/partial (init crashed, someone deleted STATE.json by
    # accident, disk lost bytes). Surface it as a block rather than
    # silently skipping — fail-safe trumps permissive. Non-mission
    # scratch folders (no lock or blueprint) are still skipped.
    if [[ ! -f "${mission_dir}STATE.json" ]]; then
      if [[ -f "${mission_dir}OUTCOME-LOCK.md" \
            || -f "${mission_dir}PROGRAM-BLUEPRINT.md" ]]; then
        blocked+=("${mid}: STATE.json missing but mission files present (corrupt)")
      fi
      continue
    fi

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

  # Neutral summary: the blocker may be corruption, not an active loop.
  printf 'Ralph blocked stop: %d mission issue(s)\n' "${#blocked[@]}" >&2
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
