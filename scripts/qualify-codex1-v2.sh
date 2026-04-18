#!/usr/bin/env bash
# Codex1 V2 live qualification driver.
#
# This script is the T42 artefact. It prepares a tempdir mission and then
# HANDS OVER to the operator, who invokes $autopilot against the mission
# via their installed Codex or Claude Code runner. The receipt is written
# by the live run, not by this script — simulation via direct CLI calls
# is explicitly forbidden (it reintroduces the V1 "proxy backend" failure
# mode the retrospective named).
#
# Usage:
#   scripts/qualify-codex1-v2.sh prepare            # → prints mission dir and instructions
#   scripts/qualify-codex1-v2.sh verify <receipt>   # → exits 0 iff receipt is valid
#
# Environment:
#   CODEX1_BIN       — path to the codex1 (or codex1-v2) binary
#   QUALIFICATION_ID — suffix on the tempdir mission id (default: timestamp)

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEMPLATE="${REPO_ROOT}/docs/qualification/codex1-v2-e2e-receipt-template.md"
RECEIPT_FINAL="${REPO_ROOT}/docs/qualification/codex1-v2-e2e-receipt.md"

BIN="${CODEX1_BIN:-}"
if [[ -z "${BIN}" ]]; then
  if command -v codex1 >/dev/null 2>&1; then
    BIN=codex1
  elif command -v codex1-v2 >/dev/null 2>&1; then
    BIN=codex1-v2
  else
    echo "qualify: no codex1 binary on PATH (set CODEX1_BIN)" >&2
    exit 2
  fi
fi

prepare() {
  local ts="${QUALIFICATION_ID:-$(date +%s)}"
  local mission_id="qual-${ts}"
  local tempdir
  tempdir="$(mktemp -d -t codex1-qual-XXXXXX)"

  "${BIN}" init \
    --mission "${mission_id}" \
    --title "Codex1 V2 end-to-end qualification" \
    --repo-root "${tempdir}" \
    --json >/dev/null

  cat <<EOF
Codex1 V2 qualification tempdir prepared.

  MISSION_ID:      ${mission_id}
  REPO_ROOT:       ${tempdir}

Next steps (run these in your Codex or Claude Code session):

  1. cd ${tempdir}
  2. Invoke the $autopilot skill against this mission. Ralph must be
     wired to ${REPO_ROOT}/scripts/ralph-status-hook.sh and the runner
     must observe stop_policy.allow_stop.
  3. When the session reaches verdict: complete + terminality: terminal,
     copy ${TEMPLATE} to ${RECEIPT_FINAL} and fill in the required markers:
       - skill_invocation: autopilot
       - ralph_hook: passed
       - verdict: complete
       - mission_id: ${mission_id}
     plus the session transcript or log reference.
  4. Run:
       ${REPO_ROOT}/scripts/qualify-codex1-v2.sh verify ${RECEIPT_FINAL}

Simulation by direct CLI calls is explicitly forbidden — the live run
must exercise $autopilot through the real skill runner so Ralph,
reviewer subagents, and skill composition are all exercised.
EOF
}

verify() {
  local receipt="$1"
  if [[ -z "${receipt}" || ! -f "${receipt}" ]]; then
    echo "qualify: receipt file not found at ${receipt}" >&2
    exit 1
  fi
  local errs=0
  for marker in \
      "skill_invocation: autopilot" \
      "ralph_hook: passed" \
      "verdict: complete"
  do
    if ! grep -q "${marker}" "${receipt}"; then
      echo "qualify: receipt missing required marker: ${marker}" >&2
      errs=$((errs + 1))
    fi
  done
  if [[ "${errs}" -gt 0 ]]; then
    echo "qualify: receipt at ${receipt} is INVALID (${errs} marker(s) missing)" >&2
    exit 1
  fi
  echo "qualify: receipt at ${receipt} VALIDATED."
}

if [[ $# -lt 1 ]]; then
  echo "usage: qualify-codex1-v2.sh prepare | verify <receipt-path>" >&2
  exit 2
fi

case "$1" in
  prepare) prepare ;;
  verify)
    if [[ $# -lt 2 ]]; then
      echo "usage: qualify-codex1-v2.sh verify <receipt-path>" >&2
      exit 2
    fi
    verify "$2"
    ;;
  *)
    echo "unknown subcommand: $1" >&2
    exit 2
    ;;
esac
