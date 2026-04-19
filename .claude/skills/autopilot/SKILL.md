---
name: autopilot
description: Codex1 V2 full-mission orchestration. Use when the user invokes $autopilot, or wants a mission driven end-to-end without manually invoking each sub-skill.
---

# $autopilot (Codex1 V2)

Compose `$clarify` → `$plan` → `$execute` → `$review-loop` and drive
`codex1 mission-close check` + `codex1 mission-close complete` at the end.
`$autopilot` owns the parent loop (`mode: autopilot`); pause via `$close`.

## When to use

- The user invokes `$autopilot`.
- The mission is well-scoped enough that the user wants hands-off
  progression to terminal complete.

## Binary resolver

Every skill starts by resolving the V2 `codex1` binary to `$CODEX1`.

```bash
CODEX1="$("${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/resolve-codex1-bin")" || {
  echo "V2 codex1 not found. Set CODEX1_REPO_ROOT=<codex1 checkout> or build with: cargo build -p codex1 --release" >&2
  exit 1
}
```

Use `"$CODEX1"` for every `codex1` invocation below.

## Composition

```
  $clarify ─┐
            │  (OUTCOME-LOCK: ratified)
  $plan ────┤
            │  (PROGRAM-BLUEPRINT: tasks, graph_revision bumped if replan)
  $execute ─┤ ◀─ $review-loop  (alternates until all tasks review_clean)
            │
  "$CODEX1" review open-mission-close --profiles mission_close
  "$CODEX1" review submit (reviewer output)
  "$CODEX1" review close (clean)
  "$CODEX1" mission-close check     ◀─ verdict continue_required until clean
  "$CODEX1" mission-close complete  ◀─ verdict flips to complete, terminality terminal
```

## Steps

1. Activate the autopilot loop:
   ```bash
   "$CODEX1" parent-loop activate --mission <id> --mode autopilot --json
   ```

2. Run `$clarify` if the lock is still `draft`; run `$plan` if the DAG is
   empty or needs replanning.

3. Iterate: while `"$CODEX1" task next` emits `start_task` or `review_open`,
   run `$execute` or `$review-loop` accordingly. Monitor for mandatory
   replan triggers:
   ```bash
   "$CODEX1" replan check --mission <id> --json
   ```
   If `mandatory_triggers` is non-empty, invoke `$plan` to replan before
   continuing.

4. When `"$CODEX1" status` emits `next_action.kind: mission_close_check`:
   ```bash
   "$CODEX1" review open-mission-close --mission <id> --profiles mission_close --json
   ```
   Dispatch a reviewer subagent against the bundle; submit the output;
   close the bundle. (Reviewer outputs must bind to the mission-close
   bundle's `evidence_snapshot_hash`.)

5. Call `"$CODEX1" mission-close check`; if `can_close: true`, call
   `"$CODEX1" mission-close complete`. Status should now show `verdict:
   complete`, `terminality: terminal`.

6. Deactivate the loop:
   ```bash
   "$CODEX1" parent-loop deactivate --mission <id> --json
   ```

## Advisor checkpoints (non-formal)

At strategic transitions — before `$plan` after `$clarify`, before
opening a mission-close bundle, before `complete` — the parent MAY
invoke an advisor/CritiqueScout and record its summary via
`advisor::append_note`. Advisor output is **not** review evidence and
does not count toward any bundle's cleanliness.

## Stop boundaries

- `$autopilot` must never self-review. The mission-close bundle's
  reviewer outputs must come from a reviewer role; parent role
  submissions are refused.
- On any `verdict: needs_user` or `blocked`, `$autopilot` surfaces the
  envelope and pauses (not deactivate — pause preserves the loop for
  resume after the user decides).
- `$autopilot` calls `mission-close complete` **only** after `check`
  returns `can_close: true`.
- Ralph blocks stop while the autopilot loop is active and not paused.
- `$close` pauses autopilot; `$autopilot` resumes continue the mission.

### Ralph parent-lane authority (Rounds 13–15 P1)

Ralph's Stop-blocking only fires in the session that owns the
parent-lane lease at `.codex1/parent-session.json`. The lease is
written by the `SessionStart` hook
(`scripts/ralph-session-lease.sh claim`) and released by `SessionEnd`;
both are wired in `.codex/hooks.json` / `.claude/hooks.json`.

**Round 15 P1 redesign**: the lease is **first-claim-wins with
PID-based staleness**. No env gate is required — the default install
works out of the box. The lease records `{session_id, pid,
claimed_at}` where `pid` is the Claude CLI process. Subsequent
`SessionStart` invocations:

- If their session_id matches the lease → idempotent refresh.
- If the lease's pid is still alive (the parent is running) → back
  off. Secondary sessions cannot steal.
- If the lease's pid is dead (the parent crashed) → take over as
  stale recovery.

The Stop hook treats a dead-pid lease as no-lease and exits 0 — a
ghost Ralph doesn't block. If hook stdin lacks `session_id` (Codex
Desktop, stripped payload), the hook falls through to scanning
(fail-closed) so Ralph doesn't silently disable itself. If the
deployment hasn't wired `SessionStart`/`SessionEnd`, no session
blocks — the hook fails open rather than punishing every lane.

## Example one-shot

```bash
"$CODEX1" parent-loop activate --mission demo --mode autopilot --json
# ...run $clarify, $plan, $execute/$review-loop loops...
"$CODEX1" review open-mission-close --mission demo --profiles mission_close --json
# ...submit reviewer output, close bundle...
"$CODEX1" mission-close check    --mission demo --json   # can_close: true
"$CODEX1" mission-close complete --mission demo --json   # phase: complete
"$CODEX1" parent-loop deactivate --mission demo --json
"$CODEX1" status --mission demo --json                   # verdict: complete
```
