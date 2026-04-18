#!/usr/bin/env bash
# Codex1 V2 Ralph status hook.
#
# Reads `codex1 status --mission <id> --json` and enforces the only
# contract Ralph owns: if the CLI says the parent loop is active and not
# paused, stop is blocked. Everything else is a pass-through.
#
# Usage:
#   ralph-status-hook.sh <mission-id> [--repo-root <path>]
#
# Exit codes:
#   0 — stop allowed; Ralph may let the current thread stop normally.
#   1 — stop blocked; print display_message to stderr for the parent.
#   2 — status invocation itself failed; treat as blocking (fail-safe).
#
# Environment:
#   CODEX1_BIN — path to the codex1 binary (default: `codex1` on PATH, or
#                `codex1-v2` during Wave 4 development).

set -eu

if [[ $# -lt 1 ]]; then
  echo "usage: ralph-status-hook.sh <mission-id> [--repo-root <path>]" >&2
  exit 2
fi

MISSION="$1"
shift

REPO_ROOT_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      REPO_ROOT_ARGS=(--repo-root "$2")
      shift 2
      ;;
    *)
      echo "ralph-status-hook: unknown arg $1" >&2
      exit 2
      ;;
  esac
done

# Pick the binary. Prefer the V2-development name during the build;
# fall back to the renamed `codex1` post-cutover.
BIN="${CODEX1_BIN:-}"
if [[ -z "${BIN}" ]]; then
  if command -v codex1 >/dev/null 2>&1; then
    BIN=codex1
  elif command -v codex1-v2 >/dev/null 2>&1; then
    BIN=codex1-v2
  else
    echo "ralph-status-hook: no codex1 binary on PATH (set CODEX1_BIN)" >&2
    exit 2
  fi
fi

# Call the CLI. Capture both stdout (JSON envelope) and exit code.
if ! OUT=$("${BIN}" --json "${REPO_ROOT_ARGS[@]}" status --mission "${MISSION}" 2>/dev/null); then
  echo "ralph-status-hook: ${BIN} status failed for mission ${MISSION}" >&2
  exit 2
fi

# Parse stop_policy.allow_stop. Use a small Python snippet if available for
# portable JSON parsing; fall back to grep-based extraction.
ALLOW_STOP=""
REASON=""
MSG=""
if command -v python3 >/dev/null 2>&1; then
  read -r ALLOW_STOP REASON MSG <<<"$(python3 - "${OUT}" <<'PY'
import json, sys
env = json.loads(sys.argv[1])
allow = "true" if env.get("stop_policy", {}).get("allow_stop") else "false"
reason = env.get("stop_policy", {}).get("reason", "")
msg = env.get("next_action", {}).get("display_message", "")
# Use tab as separator so the shell read can split on whitespace safely.
print(f"{allow}\t{reason}\t{msg}")
PY
)"
else
  # Crude fallback: look for "allow_stop":true/false in the JSON.
  if echo "${OUT}" | grep -q '"allow_stop":true'; then
    ALLOW_STOP=true
  else
    ALLOW_STOP=false
  fi
  REASON="$(echo "${OUT}" | sed -n 's/.*"reason":"\([^"]*\)".*/\1/p' | head -n1)"
  MSG="$(echo "${OUT}" | sed -n 's/.*"display_message":"\([^"]*\)".*/\1/p' | head -n1)"
fi

if [[ "${ALLOW_STOP}" == "true" ]]; then
  exit 0
fi

# Stop blocked.
printf 'Ralph blocked stop: %s\n' "${REASON:-active_parent_loop}" >&2
if [[ -n "${MSG}" ]]; then
  printf '%s\n' "${MSG}" >&2
fi
exit 1
