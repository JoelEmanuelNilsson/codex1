#!/usr/bin/env bash
# Ralph Stop hook for Codex1.
#
# Reads `codex1 status --json` and blocks Stop (exit 2) only when a loop is
# active-and-unpaused and stop.allow == false. Otherwise exits 0.
#
# Requires: codex1 on PATH, jq.
# Never inspects mission files directly. Always goes through the CLI.

set -euo pipefail

# Codex passes hook input via stdin as JSON; we don't currently consume it,
# but drain it so the pipe doesn't stall.
if [ -t 0 ]; then : ; else cat > /dev/null || true; fi

CODEX1=${CODEX1_BIN:-codex1}
STATUS_ARGS=(status --json)
if [ -n "${CODEX1_REPO_ROOT:-}" ]; then
  STATUS_ARGS+=(--repo-root "$CODEX1_REPO_ROOT")
fi
if [ -n "${CODEX1_MISSION:-}" ]; then
  STATUS_ARGS+=(--mission "$CODEX1_MISSION")
fi

if ! command -v "$CODEX1" >/dev/null 2>&1; then
  echo "ralph-stop-hook: codex1 not on PATH; allowing Stop" >&2
  exit 0
fi

# Ask status in the current repo. If no mission resolves, status returns
# stop.allow=true; exit 0 so Stop is allowed.
status_json="$("$CODEX1" "${STATUS_ARGS[@]}" 2>/dev/null || true)"
if [ -z "$status_json" ]; then
  echo "ralph-stop-hook: empty status output; allowing Stop" >&2
  exit 0
fi

# Prefer jq when available; fallback to grep if jq is missing (degraded).
#
# Note: we read `.data.stop.allow` raw (no `//` default) because the jq `//`
# alternative-operator treats literal `false` as "missing" and would flip a
# real block into an allow. We handle the null/missing case explicitly below.
if command -v jq >/dev/null 2>&1; then
  ok="$(printf '%s' "$status_json" | jq -r '.ok' 2>/dev/null || true)"
  code="$(printf '%s' "$status_json" | jq -r '.code // empty' 2>/dev/null || true)"
  ambiguous="$(printf '%s' "$status_json" | jq -r '.context.ambiguous // false' 2>/dev/null || echo false)"
  status_message="$(printf '%s' "$status_json" | jq -r '.message // empty' 2>/dev/null || true)"
  if [ "$ok" = "false" ] && [ "$code" = "MISSION_NOT_FOUND" ] && [ "$ambiguous" = "true" ]; then
    echo "ralph-stop-hook: ambiguous Codex1 mission; set CODEX1_MISSION or CODEX1_REPO_ROOT" >&2
    exit 2
  fi
  if [ "$ok" = "false" ]; then
    if [ -n "$status_message" ]; then
      echo "ralph-stop-hook: codex1 status failed: $status_message" >&2
    else
      echo "ralph-stop-hook: codex1 status failed with code=$code" >&2
    fi
    exit 2
  fi
  allow="$(printf '%s' "$status_json" | jq -r '.data.stop.allow' 2>/dev/null || true)"
  reason="$(printf '%s' "$status_json" | jq -r '.data.stop.reason // "idle"' 2>/dev/null || echo idle)"
  message="$(printf '%s' "$status_json" | jq -r '.data.stop.message // ""' 2>/dev/null || echo "")"
else
  # Rough fallback parsing; jq is strongly preferred.
  if printf '%s' "$status_json" | grep -q '"code"[[:space:]]*:[[:space:]]*"MISSION_NOT_FOUND"' \
    && printf '%s' "$status_json" | grep -q '"ambiguous"[[:space:]]*:[[:space:]]*true'; then
    echo "ralph-stop-hook: ambiguous Codex1 mission; set CODEX1_MISSION or CODEX1_REPO_ROOT" >&2
    exit 2
  fi
  if printf '%s' "$status_json" | grep -q '"ok"[[:space:]]*:[[:space:]]*false'; then
    echo "ralph-stop-hook: codex1 status returned an error envelope; blocking Stop" >&2
    exit 2
  fi
  allow="$(printf '%s' "$status_json" | grep -o '"allow"[[:space:]]*:[[:space:]]*\(true\|false\)' | head -n1 | awk -F: '{print $2}' | tr -d ' ' || true)"
  reason="unknown"
  message="jq not installed - degraded parse"
fi

case "$allow" in
  true)
    exit 0
    ;;
  false)
    echo "ralph-stop-hook: blocking Stop - reason=$reason" >&2
    [ -n "$message" ] && echo "ralph-stop-hook: $message" >&2
    exit 2
    ;;
  *)
    echo "ralph-stop-hook: could not parse stop.allow from status JSON; allowing Stop" >&2
    exit 0
    ;;
esac
