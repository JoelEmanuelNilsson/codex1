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
# Threat model (intentional and reviewed):
#   The verifier is offline. It layers (1) field-level receipt checks,
#   (2) live `codex1 validate` against mission_dir, (3) live
#   `codex1 status` (must report verdict=complete/terminality=terminal/
#   phase=complete), (4) ordered-event-trail check on events.jsonl, and
#   (5) a clean mission_close review bundle. A forger who hand-authors a
#   mission dir passing all five has effectively re-implemented the V2
#   state machine; the artifact is operationally equivalent to a real
#   run. Cryptographic proof-of-origin (signed runner tokens, etc.) is
#   out of scope and NOT a review finding — see
#   docs/codex1-v2-operator-guide.md "Verifier threat model".
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

  local mission_dir="${tempdir}/PLANS/${mission_id}"

  cat <<EOF
Codex1 V2 qualification tempdir prepared.

  MISSION_ID:      ${mission_id}
  REPO_ROOT:       ${tempdir}
  MISSION_DIR:     ${mission_dir}

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
       - mission_dir: ${mission_dir}
     plus the session transcript or log reference.
  4. DO NOT delete ${tempdir} — the verifier opens ${mission_dir} and
     cross-checks STATE.json, events.jsonl, and the review bundles
     against the receipt. A fabricated receipt pointing at a missing
     or incomplete mission is rejected.
  5. Run:
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

  # Round 4: the verifier now also invokes the V2 binary's own `validate`
  # and `status` against the mission dir. A fabricated mission that skips
  # any V2 invariant (schema drift, event-seq > state_revision, STATE
  # mission_id mismatch, missing clean mission-close bundle) will fail
  # those subprocess checks even if the file-level checks pass.
  local bin
  bin="$(resolve_bin)" || exit $?

  # Structural + mission-grounded validation lives in Python so the
  # JSON-aware checks are robust (sentinel detection, string length,
  # exact-value matching, STATE/events/bundle cross-checks, subprocess
  # validate + status enforcement).
  python3 - "${receipt}" "${bin}" <<'PY'
import json
import os
import re
import subprocess
import sys
from pathlib import Path

path = sys.argv[1]
bin_path = sys.argv[2]  # Round 4: resolved V2 binary for subprocess checks.
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
    "mission_dir",
    "operator",
    "completed_at",
    "session_transcript_excerpt",
]

# Required numeric fields: must be non-zero integers. A forger who
# writes task_count: 0 or final_state_revision: 0 gets rejected here
# before the cross-check even runs.
required_positive_int = [
    "final_state_revision",
    "final_event_seq",
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

for key in required_positive_int:
    if key not in data:
        errs.append(f"missing required field {key!r}")
    elif not isinstance(data[key], int) or isinstance(data[key], bool) or data[key] <= 0:
        errs.append(
            f"field {key!r} must be a positive integer (got {data.get(key)!r})"
        )

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

# Short-circuit if the structural pass already failed — the mission
# cross-check below assumes `mission_dir`, `mission_id`, and numeric
# fields are sane.
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
    print("Required positive-integer fields:", file=sys.stderr)
    for k in required_positive_int:
        print(f"  {k}", file=sys.stderr)
    sys.exit(1)

# ---- Mission-grounded cross-check ----
#
# A forger can write a receipt, but they cannot easily fabricate a
# mission directory whose STATE.json, events.jsonl, and review bundles
# all agree with the receipt while also obeying the V2 state-machine
# invariants (monotonic seq, matching state_revision, required
# event-stream milestones). That is the bar the verifier enforces here.

mission_dir = Path(data["mission_dir"])
mission_id = data["mission_id"]
final_state_revision = int(data["final_state_revision"])
final_event_seq = int(data["final_event_seq"])

if not mission_dir.is_absolute():
    print(
        f"qualify: mission_dir must be an absolute path (got {mission_dir!r}). "
        "Use the absolute path of PLANS/<mission_id>/ inside the qualification tempdir.",
        file=sys.stderr,
    )
    sys.exit(1)

if not mission_dir.is_dir():
    print(
        f"qualify: mission_dir {mission_dir} does not exist or is not a directory. "
        "Preserve the qualification tempdir until `verify` runs; the verifier "
        "opens STATE.json, events.jsonl, and reviews/ to cross-check the receipt.",
        file=sys.stderr,
    )
    sys.exit(1)

# 1) STATE.json
state_path = mission_dir / "STATE.json"
if not state_path.is_file():
    print(f"qualify: STATE.json not found at {state_path}", file=sys.stderr)
    sys.exit(1)
try:
    state = json.loads(state_path.read_text(encoding="utf-8"))
except json.JSONDecodeError as e:
    print(f"qualify: STATE.json is not valid JSON: {e}", file=sys.stderr)
    sys.exit(1)

state_errs = []
if state.get("mission_id") != mission_id:
    state_errs.append(
        f"STATE.json.mission_id={state.get('mission_id')!r} != receipt.mission_id={mission_id!r}"
    )
if state.get("phase") != "complete":
    state_errs.append(
        f"STATE.json.phase={state.get('phase')!r} but receipt claims verdict: complete "
        f"(mission-close complete must have run)"
    )
if state.get("state_revision") != final_state_revision:
    state_errs.append(
        f"STATE.json.state_revision={state.get('state_revision')!r} "
        f"!= receipt.final_state_revision={final_state_revision!r}"
    )

# 2) events.jsonl — every line parses, last seq matches, required
#    milestone events are present in monotonic order.
events_path = mission_dir / "events.jsonl"
if not events_path.is_file():
    print(f"qualify: events.jsonl not found at {events_path}", file=sys.stderr)
    sys.exit(1)

events = []
try:
    for lineno, raw in enumerate(events_path.read_text(encoding="utf-8").splitlines(), 1):
        raw = raw.strip()
        if not raw:
            continue
        events.append((lineno, json.loads(raw)))
except json.JSONDecodeError as e:
    print(f"qualify: events.jsonl has malformed JSON: {e}", file=sys.stderr)
    sys.exit(1)

event_errs = []
if not events:
    event_errs.append("events.jsonl is empty; a real $autopilot run must emit events")
else:
    last_seq = events[-1][1].get("seq")
    sr = state.get("state_revision")
    # last seq must equal state_revision (normal) or state_revision - 1
    # (one-line lag permitted by the V2 validate contract).
    if not (isinstance(last_seq, int) and isinstance(sr, int) and last_seq in (sr, sr - 1)):
        event_errs.append(
            f"events.jsonl last seq={last_seq!r} is not consistent with "
            f"STATE.json.state_revision={sr!r} (allowed: equal or lag by 1)"
        )
    if last_seq != final_event_seq:
        event_errs.append(
            f"events.jsonl last seq={last_seq!r} != receipt.final_event_seq={final_event_seq!r}"
        )

    # Required milestone events (actual emitted names, per V2 source).
    def find_first(kind_pred, extra_pred=None):
        for _, ev in events:
            if kind_pred(ev.get("kind")):
                if extra_pred is None or extra_pred(ev):
                    return ev
        return None

    autopilot_activated = find_first(
        lambda k: k == "parent_loop_activated",
        lambda ev: ev.get("mode") == "autopilot",
    )
    if autopilot_activated is None:
        event_errs.append(
            "no parent_loop_activated event with mode=autopilot found "
            "(proof that $autopilot actually activated the loop)"
        )

    required_kinds = [
        "task_started",
        "task_finished",
        "review_opened",
        "review_closed",
        "mission_closed",
    ]
    for kind in required_kinds:
        if find_first(lambda k, _k=kind: k == _k) is None:
            event_errs.append(
                f"no {kind!r} event found in events.jsonl (required for a real $autopilot run)"
            )

    # Monotonic + ordered milestones: parent_loop_activated(autopilot) → task_started
    # → task_finished → review_opened → review_closed → mission_closed.
    def first_seq_where(kind_pred, extra_pred=None):
        for _, ev in events:
            if kind_pred(ev.get("kind")):
                if extra_pred is None or extra_pred(ev):
                    return ev.get("seq")
        return None

    order_expected = [
        ("parent_loop_activated(autopilot)",
         first_seq_where(lambda k: k == "parent_loop_activated",
                         lambda ev: ev.get("mode") == "autopilot")),
        ("task_started", first_seq_where(lambda k: k == "task_started")),
        ("task_finished", first_seq_where(lambda k: k == "task_finished")),
        ("review_opened", first_seq_where(lambda k: k == "review_opened")),
        ("review_closed", first_seq_where(lambda k: k == "review_closed")),
        ("mission_closed", first_seq_where(lambda k: k == "mission_closed")),
    ]
    prev_name, prev_seq = None, None
    for name, seq in order_expected:
        if seq is None:
            continue  # already reported above
        if prev_seq is not None and seq < prev_seq:
            event_errs.append(
                f"event order violated: first {name} at seq {seq} "
                f"precedes first {prev_name} at seq {prev_seq}"
            )
        prev_name, prev_seq = name, seq

# 3) Review bundles — at least one mission_close target with status clean.
reviews_dir = mission_dir / "reviews"
bundle_errs = []
if not reviews_dir.is_dir():
    bundle_errs.append(f"reviews/ directory not found at {reviews_dir}")
else:
    found_clean_mc = False
    for entry in sorted(reviews_dir.iterdir()):
        if entry.is_file() and entry.name.startswith("B") and entry.suffix == ".json":
            try:
                b = json.loads(entry.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                bundle_errs.append(f"bundle {entry.name} is not valid JSON: {e}")
                continue
            target = b.get("target", {})
            kind = target.get("kind") if isinstance(target, dict) else None
            status = b.get("status")
            if kind == "mission_close" and status == "clean":
                found_clean_mc = True
                break
    if not found_clean_mc:
        bundle_errs.append(
            "no clean mission_close review bundle found under reviews/B*.json "
            "(mission-close cannot be legitimately complete without one)"
        )

# 4) Ground the receipt in the V2 binary's own machinery.
#    mission_dir is <repo-root>/PLANS/<mission_id>, so the repo-root is
#    the parent-of-parent. A forger would have to produce a mission dir
#    that passes V2's real `validate` + emits a `verdict: complete,
#    terminality: terminal, phase: complete` status envelope — at which
#    point they've essentially re-run the V2 state machine by hand.
binary_errs = []
try:
    repo_root = mission_dir.parent.parent
except Exception as exc:  # noqa: BLE001
    binary_errs.append(f"cannot derive repo-root from mission_dir: {exc}")
    repo_root = None

def run_cli(args, label):
    try:
        proc = subprocess.run(
            [bin_path, "--repo-root", str(repo_root), "--json", *args],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except FileNotFoundError:
        binary_errs.append(
            f"resolved codex1 binary {bin_path!r} is not executable"
        )
        return None
    except subprocess.TimeoutExpired:
        binary_errs.append(f"codex1 {label} timed out after 30s")
        return None

    stdout = proc.stdout.strip().splitlines()
    envelope = None
    for line in reversed(stdout):
        line = line.strip()
        if not line:
            continue
        try:
            envelope = json.loads(line)
            break
        except json.JSONDecodeError:
            continue
    if envelope is None:
        binary_errs.append(
            f"codex1 {label} produced no JSON envelope on stdout "
            f"(exit={proc.returncode}, stderr={proc.stderr.strip()!r})"
        )
    return envelope

if repo_root is not None:
    validate_env = run_cli(["validate", "--mission", mission_id], "validate")
    if validate_env is not None and validate_env.get("ok") is not True:
        code = validate_env.get("code", "unknown")
        msg = validate_env.get("message", "")
        binary_errs.append(
            f"codex1 validate refuses mission {mission_id!r}: code={code} message={msg!r}"
        )

    status_env = run_cli(["status", "--mission", mission_id], "status")
    if status_env is not None:
        if status_env.get("verdict") != "complete":
            binary_errs.append(
                f"codex1 status reports verdict={status_env.get('verdict')!r} "
                f"(receipt claims complete)"
            )
        if status_env.get("terminality") != "terminal":
            binary_errs.append(
                f"codex1 status reports terminality={status_env.get('terminality')!r} "
                f"(receipt claims terminal)"
            )
        if status_env.get("phase") != "complete":
            binary_errs.append(
                f"codex1 status reports phase={status_env.get('phase')!r} "
                f"(receipt claims complete)"
            )

all_errs = state_errs + event_errs + bundle_errs + binary_errs
if all_errs:
    print(f"qualify: mission at {mission_dir} does not match receipt:", file=sys.stderr)
    for e in all_errs:
        print(f"  - {e}", file=sys.stderr)
    sys.exit(1)

print(f"qualify: receipt at {path} VALIDATED.")
print(f"         cross-checked against mission {mission_id} at {mission_dir}")
print(f"         V2 validate + status endorse the mission state")
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
