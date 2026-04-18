---
name: close
description: Pause or clear the active Codex1 Ralph parent loop so the user can discuss, redirect, or stop without uninstalling hooks.
---

# Close

Use this public skill when the user wants to stop, pause, discuss, redirect, or
leave a Ralph-governed parent loop without moving `.codex/hooks.json`.

## Owns

- Pausing or clearing `.ralph/loop-lease.json`
- Explaining the next resumable branch without mutating mission truth
- Preserving the user's ability to resume later with `$plan`, `$execute`,
  `$review-loop`, or `$autopilot`
- Freezing parent integration while allowing already-running bounded child,
  reviewer, or advisor lanes to finish and persist their bounded outputs
- Keeping `paused` distinct from `needs_user`, `hard_blocked`, and `complete`

## Workflow

1. Inspect the current lease with `codex1 internal inspect-loop-lease`.
2. If a lease exists, pause it with `codex1 internal pause-loop-lease`:

```json
{
  "paused_by": "user",
  "reason": "User invoked $close to discuss or stop the Ralph loop."
}
```

3. If the user explicitly asks to discard loop state, use
   `codex1 internal clear-loop-lease` instead.
4. Already-running bounded lanes may finish and persist their bounded outputs,
   but `$close` must not integrate those outputs or act on them. The parent must
   revalidate freshness after the user resumes with `$plan`, `$execute`,
   `$review-loop`, or `$autopilot`.
5. Do not resolve gates, repair specs, record review outcomes, compile
   packages, or continue execution as part of `$close`.
6. Report the current pending mission branch as passive status only. A paused
   loop is not a durable `needs_user` wait, not `hard_blocked`, and not
   `complete`.

## Resume

The user resumes by invoking one of the loop-owning public workflows:

- `$plan` starts or refreshes a `planning_loop` lease.
- `$execute` starts or refreshes an `execution_loop` lease.
- `$review-loop` starts or refreshes a `review_loop` lease.
- `$autopilot` starts or refreshes an `autopilot_loop` lease.

## Must Not

- uninstall or move `.codex/hooks.json`
- mark missions complete
- clear gates
- treat pausing Ralph as proof that work is done
- integrate child/reviewer/advisor outputs while the parent loop is paused
- mint new child lane authority while the parent loop is paused
- describe a paused loop as terminal, blocked, or waiting on a user decision
