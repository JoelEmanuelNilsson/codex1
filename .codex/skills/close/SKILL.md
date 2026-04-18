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
4. Do not resolve gates, repair specs, record review outcomes, compile
   packages, or continue execution as part of `$close`.
5. Report the current pending mission branch as passive status only.

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
