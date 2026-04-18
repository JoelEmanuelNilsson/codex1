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

## Composition

```
  $clarify ─┐
            │  (OUTCOME-LOCK: ratified)
  $plan ────┤
            │  (PROGRAM-BLUEPRINT: tasks, graph_revision bumped if replan)
  $execute ─┤ ◀─ $review-loop  (alternates until all tasks review_clean)
            │
  codex1 review open-mission-close --profiles mission_close
  codex1 review submit (reviewer output)
  codex1 review close (clean)
  codex1 mission-close check     ◀─ verdict continue_required until clean
  codex1 mission-close complete  ◀─ verdict flips to complete, terminality terminal
```

## Steps

1. Activate the autopilot loop:
   ```bash
   codex1 parent-loop activate --mission <id> --mode autopilot --json
   ```

2. Run `$clarify` if the lock is still `draft`; run `$plan` if the DAG is
   empty or needs replanning.

3. Iterate: while `codex1 task next` emits `start_task` or `review_open`,
   run `$execute` or `$review-loop` accordingly. Monitor for mandatory
   replan triggers:
   ```bash
   codex1 replan check --mission <id> --json
   ```
   If `mandatory_triggers` is non-empty, invoke `$plan` to replan before
   continuing.

4. When `codex1 status` emits `next_action.kind: mission_close_check`:
   ```bash
   codex1 review open-mission-close --mission <id> --profiles mission_close --json
   ```
   Dispatch a reviewer subagent against the bundle; submit the output;
   close the bundle. (Reviewer outputs must bind to the mission-close
   bundle's `evidence_snapshot_hash`.)

5. Call `codex1 mission-close check`; if `can_close: true`, call
   `codex1 mission-close complete`. Status should now show `verdict:
   complete`, `terminality: terminal`.

6. Deactivate the loop:
   ```bash
   codex1 parent-loop deactivate --mission <id> --json
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

## Example one-shot

```bash
codex1 parent-loop activate --mission demo --mode autopilot --json
# ...run $clarify, $plan, $execute/$review-loop loops...
codex1 review open-mission-close --mission demo --profiles mission_close --json
# ...submit reviewer output, close bundle...
codex1 mission-close check    --mission demo --json   # can_close: true
codex1 mission-close complete --mission demo --json   # phase: complete
codex1 parent-loop deactivate --mission demo --json
codex1 status --mission demo --json                   # verdict: complete
```
