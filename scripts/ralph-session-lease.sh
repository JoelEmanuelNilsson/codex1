#!/usr/bin/env bash
# Codex1 V2 parent-lane lease manager.
#
# Wired into Claude Code's SessionStart and SessionEnd hooks so that
# the first (parent-orchestrator) Claude session in this repo records
# its `session_id` to `.codex1/parent-session.json`. The Stop hook
# later consults the lease to decide whether the current session is
# THE parent lane or a secondary/standalone session that must not be
# blocked.
#
# Usage (invoked by Claude Code hooks, hook JSON piped on stdin):
#   ralph-session-lease.sh claim    # SessionStart
#   ralph-session-lease.sh release  # SessionEnd
#   ralph-session-lease.sh is-parent  # returns 0 if caller's session owns the lease
#
# Semantics:
#   claim     — writes {session_id, claimed_at} atomically; last writer
#               wins (a second concurrent root session takes over).
#   release   — deletes the lease if the current session owns it; no-op
#               otherwise (tolerates a racey reclaim).
#   is-parent — exit 0 if current session_id == lease.session_id; exit
#               1 otherwise (no lease, or owned by someone else). Used
#               by the Stop hook to gate its scan.
#
# The lease is NOT a general-purpose lock — it's an identity marker.
# Without this marker, the Stop hook fails open (exit 0), which is the
# correct choice for a session that never claimed the parent lane.
# Without SessionStart wired, no session ever claims, so the hook
# behaves as a no-op. Ralph setups that want stop-blocking must wire
# both hooks.

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

claim_lease() {
  local sid="$1"
  [[ -n "$sid" ]] || return 0
  mkdir -p "${LEASE_DIR}"
  # Atomic write via temp+rename. Last writer wins — the reviewer's
  # scenario of "a second root session overwrites" is intentional and
  # unambiguous at the hook layer (sessions serialize on SessionStart).
  python3 - "$sid" "$LEASE_FILE" <<'PY'
import json, os, sys, time
sid = sys.argv[1]
path = sys.argv[2]
tmp = path + ".tmp"
payload = {
    "session_id": sid,
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
    # Round 14 P1: only the parent-orchestrator lane may claim the
    # lease. Ralph (or any wrapper that wants Stop-blocking) MUST set
    # `CODEX1_PARENT_LANE=1` in the env that the Claude process runs
    # under. Without this gate, every root session's SessionStart
    # would overwrite the lease and steal parent-lane authority from
    # whoever actually owns the loop.
    #
    # Intentional design trade-off: env propagates to Agent-tool
    # subagents, so their SessionStart would also try to claim. The
    # Stop hook is NOT registered for SubagentStop, so a subagent's
    # claim is harmless (no Stop hook runs in the subagent). The
    # subagent's SessionEnd releases only if the lease's session_id
    # matches its own — which it never does, so the parent's lease
    # survives the subagent's lifecycle.
    if [[ "${CODEX1_PARENT_LANE:-0}" != "1" ]]; then
      exit 0
    fi
    sid="$(read_session_id_from_stdin)"
    claim_lease "$sid"
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
