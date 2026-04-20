---
name: close
description: >
  Pause the active Codex1 loop so the user can discuss with Codex without Ralph forcing continuation. Use when the user wants to talk, ask questions, change scope, or interrupt — `$close` is a discussion-mode boundary, not mission completion. Runs `codex1 loop pause`, engages with the user, then either `codex1 loop resume` to continue or `codex1 loop deactivate` to stop. Separately documents the terminal-close path (`codex1 close complete` after mission-close review has passed).
---

# Close

## Overview

`$close` is a discussion-mode pause. It separates two distinct concepts:

- Loop pause/resume/deactivate — this skill's main job. Lets the user talk to Codex without Ralph re-pressing "continue."
- Mission terminal close — `codex1 close complete`. Happens ONLY after mission-close review passes; normally handled by `$review-loop` plus explicit user confirmation. Terminal close is documented here so the two paths are not confused.

Default to the discussion-mode workflow. Only touch `codex1 close complete` when the distinction checks below are all satisfied.

## Distinction: `$close` is NOT mission completion

Pausing the loop is a boundary for user conversation. It leaves mission truth untouched.

Mission completion requires all of the following to agree:

- Outcome ratified (`outcome.ratified == true`).
- Plan locked (`plan.locked == true`).
- All required tasks complete or review-clean.
- Mission-close review clean (`close.review_state == Passed`).
- `codex1 --json close check` returns `ready: true` and `verdict: mission_close_review_passed`.

If any of these is missing, do not run `codex1 close complete`. Stay in discussion mode or resume execution.

## Required workflow — discussion mode

Run these steps when the user wants to pause, ask questions, change scope, or interrupt.

1. Pause the loop:

   ```bash
   codex1 --json loop pause --mission <id>
   ```

   Ralph now allows Stop. The user can type freely without Ralph re-pressing "continue."

2. Listen to the user. Answer questions. Investigate files if asked. Do not continue executing tasks, do not finish tasks, do not record reviews, and do not mutate mission truth while paused.

3. Decide with the user. Pick one:

   - Resume the mission as planned:

     ```bash
     codex1 --json loop resume --mission <id>
     ```

     Then hand back to `$execute` (or the appropriate phase skill).

   - Adjust the plan. Get explicit user agreement first, then hand to `$plan replan`. Do not edit `PLAN.yaml` mid-discussion.

   - End the active loop but keep the mission open:

     ```bash
     codex1 --json loop deactivate --mission <id>
     ```

     The mission stays open; restart later with `$autopilot`.

   - Cancel the mission permanently: deactivate as above, then stop. There is no explicit "abort" command — deactivate + no resume is enough.

## Required workflow — terminal close

Only enter this workflow when `codex1 --json status` reports:

```
verdict: mission_close_review_passed
close_ready: true
```

Then:

1. Confirm with the user in plain words that the mission should be closed. Do not close without confirmation.

2. Run:

   ```bash
   codex1 --json close complete --mission <id>
   ```

   This writes `PLANS/<id>/CLOSEOUT.md`, sets `state.phase = terminal`, and deactivates the loop.

3. Idempotent. Subsequent calls return `TERMINAL_ALREADY_COMPLETE`; treat that as success, not failure.

If `close check` reports any other verdict, do not run `close complete`. Report the blockers back to the user and return to the discussion-mode workflow or to the appropriate phase skill.

## Ralph interaction

While paused:

- `codex1 --json status` reports `loop.paused: true`, `stop.allow: true`, `stop.reason: paused`.
- Ralph does not block Stop. The user can type, interrupt, and discuss freely.

When resumed:

- `loop.paused: false` and Ralph resumes its stop-guard behavior on the active main thread.

Ralph reads `status` only. Do not try to bypass or "unstick" Ralph by editing state directly — pause and resume via the `loop` commands.

## Do not

- Do not run `codex1 close complete` before mission-close review has passed. That is a contract violation and will fail with `CLOSE_NOT_READY`.
- Do not replan, edit `PLAN.yaml`, finish tasks, or record reviews while paused for discussion. Pause is for talking, not for mutating mission truth.
- Do not treat `codex1 loop pause` or `codex1 loop deactivate` as mission completion. The mission stays open until `close complete` runs.
- Do not invent an "abort" command. Deactivate + no resume is the cancel path.
- Do not edit `.ralph/` files, `STATE.json`, or hooks to work around Ralph. Use the `loop` commands.
