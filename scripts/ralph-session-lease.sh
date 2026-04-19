#!/usr/bin/env bash
# Codex1 V2 parent-lane lease manager.
#
# Wired into Claude Code's SessionStart and SessionEnd hooks so that
# the first (parent-orchestrator) Claude session in this repo records
# its `session_id` + process pid to `.codex1/parent-session.json`. The
# Stop hook later consults the lease to decide whether the current
# session is THE parent lane or a secondary/standalone session that
# must not be blocked.
#
# Usage (invoked by Claude Code hooks, hook JSON piped on stdin):
#   ralph-session-lease.sh claim    # SessionStart
#   ralph-session-lease.sh release  # SessionEnd
#   ralph-session-lease.sh is-parent  # exit 0 iff caller owns the lease
#
# Round 15 P1 redesign: first-claim-wins with PID-based staleness.
#
#   - claim   — if no live lease exists, write {session_id, pid,
#               claimed_at} with pid = $PPID (the Claude CLI process).
#               If a lease exists and its pid is alive, back off; the
#               parent's claim is sticky. If the lease's pid is dead,
#               take over (stale recovery). Idempotent refresh if the
#               current session already owns the lease.
#   - release — delete the lease if the current session owns it; no-op
#               otherwise.
#   - is-parent — exit 0 iff current session_id matches the lease's.
#
# Why no `CODEX1_PARENT_LANE` env gate (Round 14's design)?
#   (a) The shipped hooks.json never exported it, so the default
#       install silently disabled Ralph.
#   (b) If it WERE exported, Agent-tool subagents inherit env and their
#       SessionStart would overwrite the parent's lease (last-writer-
#       wins). PID-check fixes both: no env required, and a subagent's
#       second claim sees the parent's live pid and backs off.
#
# The lease is NOT a general-purpose lock — it's an identity marker.
# Ralph setups that want stop-blocking must wire SessionStart,
# SessionEnd, and Stop hooks.

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

find_repo_root() {
  local dir="${1:-$PWD}"
  while :; do
    if [[ -d "$dir/PLANS" ]]; then
      printf '%s' "$dir"
      return 0
    fi
    local parent
    parent="$(dirname "$dir")"
    [[ "$parent" == "$dir" ]] && break
    dir="$parent"
  done
  # No PLANS/ ancestor found. Return 1 so the caller can distinguish
  # "not a codex1 repo" from "found it." Don't pollute $PWD/.codex1
  # with a stray lease when Claude is started from $HOME, /tmp, etc.
  return 1
}

if [[ -n "${CODEX1_REPO_ROOT:-}" ]]; then
  REPO_ROOT="${CODEX1_REPO_ROOT}"
elif REPO_ROOT="$(find_repo_root "$PWD")"; then
  :
else
  # Outside any codex1 mission tree — nothing to claim.
  exit 0
fi
LEASE_DIR="${REPO_ROOT}/.codex1"
LEASE_FILE="${LEASE_DIR}/parent-session.json"

read_session_id_from_stdin() {
  # Hook JSON on stdin includes session_id (contractually per Claude
  # Code v2.1+). If stdin is missing or unparseable, no session_id.
  # NOTE: uses `python3 -c` rather than a heredoc so stdin reaches
  # python (a heredoc would itself become stdin).
  python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    sid = data.get("session_id", "")
    if isinstance(sid, str):
        print(sid)
except Exception:
    pass
' 2>/dev/null || true
}

# Check whether a pid is alive. `kill -0 <pid>` exits 0 if the pid
# is a live process the user can signal. For Ralph's purpose this is
# precise enough: the parent Claude CLI is always a process this user
# owns, so a positive signal probe means the parent is still running.
pid_is_alive() {
  local pid="$1"
  [[ -n "$pid" && "$pid" =~ ^[0-9]+$ ]] || return 1
  kill -0 "$pid" 2>/dev/null
}

# Overrideable for tests. The SessionStart hook script is spawned as a
# child of the Claude CLI, so $PPID is the Claude CLI pid. Tests can
# set CODEX1_PARENT_PID to simulate a specific parent process.
current_parent_pid() {
  if [[ -n "${CODEX1_PARENT_PID:-}" ]]; then
    printf '%s' "${CODEX1_PARENT_PID}"
  else
    printf '%s' "${PPID}"
  fi
}

# Returns 0 iff the lease at $LEASE_FILE exists AND its pid is alive.
# Used by claim_lease to decide whether to back off (live owner) or
# take over (stale lease).
lease_is_live() {
  [[ -f "$LEASE_FILE" ]] || return 1
  local existing_pid
  existing_pid="$(python3 -c '
import json, sys
try:
    with open(sys.argv[1]) as f:
        data = json.load(f)
    pid = data.get("pid", "")
    print(pid)
except Exception:
    pass
' "${LEASE_FILE}" 2>/dev/null || true)"
  pid_is_alive "$existing_pid"
}

lease_owner_sid() {
  [[ -f "$LEASE_FILE" ]] || { printf ''; return 0; }
  python3 -c '
import json, sys
try:
    with open(sys.argv[1]) as f:
        data = json.load(f)
    sid = data.get("session_id", "")
    if isinstance(sid, str):
        print(sid)
except Exception:
    pass
' "${LEASE_FILE}" 2>/dev/null || true
}

claim_lease() {
  local sid="$1"
  local pid="$2"
  [[ -n "$sid" ]] || return 0
  mkdir -p "${LEASE_DIR}"
  # First-claim-wins with PID-based staleness. See the file header.
  if [[ -f "$LEASE_FILE" ]]; then
    local existing_sid
    existing_sid="$(lease_owner_sid)"
    if [[ "$existing_sid" == "$sid" ]]; then
      # Idempotent refresh: keep the lease but update claimed_at so
      # long-lived sessions don't look stale to a future heuristic.
      :
    elif lease_is_live; then
      # Another live session already owns the lease. Back off.
      return 0
    fi
    # Lease is stale (pid dead) — fall through and overwrite.
  fi
  python3 - "$sid" "$pid" "$LEASE_FILE" <<'PY'
import json, os, sys, time
sid = sys.argv[1]
pid = sys.argv[2]
path = sys.argv[3]
tmp = path + ".tmp"
payload = {
    "session_id": sid,
    "pid": int(pid) if pid.isdigit() else pid,
    "claimed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(tmp, "w") as f:
    json.dump(payload, f, indent=2)
os.replace(tmp, path)
PY
}

release_lease() {
  local sid="$1"
  [[ -n "$sid" ]] || return 0
  [[ -f "$LEASE_FILE" ]] || return 0
  python3 - "$sid" "$LEASE_FILE" <<'PY'
import json, os, sys
sid = sys.argv[1]
path = sys.argv[2]
try:
    with open(path) as f:
        cur = json.load(f)
except Exception:
    sys.exit(0)
if cur.get("session_id") == sid:
    os.remove(path)
PY
}

is_parent() {
  local sid="$1"
  [[ -n "$sid" && -f "$LEASE_FILE" ]] || return 1
  python3 - "$sid" "$LEASE_FILE" <<'PY' && return 0 || return 1
import json, sys
sid = sys.argv[1]
path = sys.argv[2]
try:
    with open(path) as f:
        cur = json.load(f)
except Exception:
    sys.exit(1)
sys.exit(0 if cur.get("session_id") == sid else 1)
PY
}

cmd="${1:-}"
shift || true

case "$cmd" in
  claim)
    sid="$(read_session_id_from_stdin)"
    pid="$(current_parent_pid)"
    claim_lease "$sid" "$pid"
    ;;
  release)
    sid="$(read_session_id_from_stdin)"
    release_lease "$sid"
    ;;
  is-parent)
    # Accept session_id from stdin JSON (hook integration) or --sid
    # argument (test harnesses). `local` is illegal at script scope,
    # so use a plain variable.
    check_sid=""
    if [[ "${1:-}" == "--sid" ]]; then
      check_sid="${2:-}"
    else
      check_sid="$(read_session_id_from_stdin)"
    fi
    if is_parent "$check_sid"; then
      exit 0
    else
      exit 1
    fi
    ;;
  *)
    echo "usage: $0 {claim|release|is-parent [--sid <id>]}" >&2
    exit 2
    ;;
esac
