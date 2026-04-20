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

if ! command -v "$CODEX1" >/dev/null 2>&1; then
  echo "ralph-stop-hook: codex1 not on PATH; allowing Stop" >&2
  exit 0
fi

# Ask status in the current repo. If no mission resolves, status returns
# stop.allow=true; exit 0 so Stop is allowed.
status_json="$("$CODEX1" status --json 2>/dev/null || true)"
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
  allow="$(printf '%s' "$status_json" | jq -r '.data.stop.allow' 2>/dev/null || true)"
  reason="$(printf '%s' "$status_json" | jq -r '.data.stop.reason // "idle"' 2>/dev/null || echo idle)"
  message="$(printf '%s' "$status_json" | jq -r '.data.stop.message // ""' 2>/dev/null || echo "")"
else
  # Rough fallback parsing; jq is strongly preferred.
  allow="$(printf '%s' "$status_json" | grep -o '"allow"[[:space:]]*:[[:space:]]*\(true\|false\)' | head -n1 | awk -F: '{print $2}' | tr -d ' ')"
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
