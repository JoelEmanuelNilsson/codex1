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

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEMPLATE="${REPO_ROOT}/docs/qualification/codex1-v2-e2e-receipt-template.md"
RECEIPT_FINAL="${REPO_ROOT}/docs/qualification/codex1-v2-e2e-receipt.md"

# shellcheck source=lib/resolve-codex1.sh
source "${SCRIPT_DIR}/lib/resolve-codex1.sh"

# `verify` doesn't need a binary; resolve lazily only in `prepare`.
resolve_bin() {
  if [[ -n "${__RESOLVED_BIN:-}" ]]; then
    echo "${__RESOLVED_BIN}"
    return 0
  fi
  __RESOLVED_BIN="$(resolve_codex1 "${REPO_ROOT}")" || return $?
  echo "${__RESOLVED_BIN}"
}

prepare() {
  local ts="${QUALIFICATION_ID:-$(date +%s)}"
  local mission_id="qual-${ts}"
  local tempdir
  tempdir="$(mktemp -d -t codex1-qual-XXXXXX)"

  local bin
  bin="$(resolve_bin)" || exit $?

  "${bin}" init \
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
  2. Invoke the \$autopilot skill against this mission. Ralph must be
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
must exercise \$autopilot through the real skill runner so Ralph,
reviewer subagents, and skill composition are all exercised.
EOF
}

verify() {
  local receipt="$1"
  if [[ -z "${receipt}" || ! -f "${receipt}" ]]; then
    echo "qualify: receipt file not found at ${receipt}" >&2
    exit 1
  fi

  # Refuse the template by filename.
  local base
  base="$(basename "${receipt}")"
  if [[ "${base}" == *-template.md ]]; then
    echo "qualify: ${receipt} is the shipped template — copy it to" >&2
    echo "        docs/qualification/codex1-v2-e2e-receipt.md and fill in" >&2
    echo "        the JSON block with real session data before verifying." >&2
    exit 1
  fi

  # Refuse receipts whose first H1 still contains the word "Template".
  local first_h1
  first_h1="$(grep -m1 '^# ' "${receipt}" || true)"
  if [[ "${first_h1}" == *Template* ]]; then
    echo "qualify: receipt's first H1 still says 'Template' — this looks" >&2
    echo "        like the unfilled template. Remove '(Template)' from" >&2
    echo "        the H1 and fill in the JSON block." >&2
    exit 1
  fi

  if ! command -v python3 >/dev/null 2>&1; then
    echo "qualify: python3 is required to validate the receipt JSON" >&2
    exit 2
  fi

  # Structural validation lives in Python so the JSON-aware checks are
  # robust (sentinel detection, string length, exact-value matching).
  python3 - "${receipt}" <<'PY'
import json
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# Find the first ```json ... ``` fenced block.
match = re.search(r"```json\s*\n(.*?)\n```", content, re.DOTALL)
if not match:
    print(f"qualify: no fenced ```json``` block found in {path}", file=sys.stderr)
    sys.exit(1)
try:
    data = json.loads(match.group(1))
except json.JSONDecodeError as e:
    print(f"qualify: JSON block is not valid JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Required-exact fields: must be present and equal the expected value.
required_exact = {
    "skill_invocation": "autopilot",
    "ralph_hook": "passed",
    "verdict": "complete",
    "terminality": "terminal",
}

# Required non-placeholder fields: must be present, non-empty strings,
# and not equal to a placeholder sentinel.
required_non_placeholder = [
    "mission_id",
    "operator",
    "completed_at",
    "session_transcript_excerpt",
]

PLACEHOLDER = "TODO-FILL-IN"

def is_placeholder(value):
    if not isinstance(value, str):
        return False
    stripped = value.strip()
    if not stripped:
        return True
    if stripped == PLACEHOLDER:
        return True
    if stripped.startswith("<TODO"):
        return True
    return False

errs = []

for key, expected in required_exact.items():
    if key not in data:
        errs.append(f"missing required field {key!r}")
    elif data[key] != expected:
        errs.append(
            f"field {key!r} must equal {expected!r} (got {data[key]!r})"
        )

for key in required_non_placeholder:
    if key not in data:
        errs.append(f"missing required field {key!r}")
    elif is_placeholder(data[key]):
        errs.append(f"field {key!r} still has placeholder value {data[key]!r}")

# Extra semantic check: session transcript must be substantial.
excerpt = data.get("session_transcript_excerpt")
if (
    isinstance(excerpt, str)
    and not is_placeholder(excerpt)
    and len(excerpt.strip()) < 40
):
    errs.append(
        "field 'session_transcript_excerpt' too short "
        f"(>= 40 chars of real transcript required; got {len(excerpt.strip())})"
    )

if errs:
    print(f"qualify: receipt at {path} is INVALID:", file=sys.stderr)
    for e in errs:
        print(f"  - {e}", file=sys.stderr)
    print("", file=sys.stderr)
    print("Required machine fields + expected values:", file=sys.stderr)
    for k, v in required_exact.items():
        print(f"  {k}: {v}", file=sys.stderr)
    print("Required non-placeholder fields:", file=sys.stderr)
    for k in required_non_placeholder:
        print(f"  {k}", file=sys.stderr)
    sys.exit(1)

print(f"qualify: receipt at {path} VALIDATED.")
PY
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
