---
name: autopilot
description: >
  Take a Codex1 mission from start to terminal close autonomously, pausing only for genuine user
  input (scope, risk, money, deploy, destructive operations). Use when the user asks to "just run
  the whole thing" or wants the mission driven end-to-end. Composes $clarify, $plan, $execute,
  $review-loop, and $close around `codex1 status --json` as the single source of truth. Handles
  replans, repairs, mission-close review, and terminal close automatically.
---

# Autopilot

## Overview

`$autopilot` drives a Codex1 mission from clarify to terminal close without requiring the user to
invoke each sub-skill by hand. The loop is simple:

1. Read `codex1 status --json` (single source of truth).
2. Dispatch on `next_action.kind` to the appropriate sub-skill or CLI call.
3. Stop only on terminal completion, a `$close` pause, an error, or a pause trigger that requires
   genuine user input.

`$autopilot` never writes STATE.json, PLAN.yaml, or OUTCOME.md directly. Every mutation goes
through the CLI.

## Preconditions

A mission directory must already exist. If the user has not initialized one, ask for a mission id
and run:

```bash
codex1 init --mission <id>
```

Then begin the loop.

## Main loop

Each iteration re-reads status. Never cache verdicts between iterations.

### Step 1 - Read status

```bash
codex1 --json status --mission <id>
```

Parse `verdict`, `next_action`, `loop`, `stop`, and `close_ready`. Ignore any chat-derived
assumption about state; trust only the envelope.

### Step 2 - Dispatch on `next_action.kind`

| `next_action.kind`          | Autopilot action                                                |
|-----------------------------|-----------------------------------------------------------------|
| `clarify`                   | Invoke `$clarify`, then return to Step 1.                       |
| `plan`                      | Choose level (see below), invoke `$plan`, return to Step 1.     |
| `run_task` / `run_wave`     | Invoke `$execute` for one step, return to Step 1.               |
| `run_review`                | Invoke `$review-loop` (planned task mode), return to Step 1.    |
| `mission_close_review`      | Invoke `$review-loop` (mission-close mode), return to Step 1.   |
| `repair`                    | Invoke `$execute` with the repair task, return to Step 1.       |
| `replan`                    | Invoke `$plan replan`, return to Step 1.                        |
| `close`                     | Confirm with the user, then `codex1 close complete`, then stop. |
| `blocked`                   | Escalate to the user; do not loop.                              |
| `closed`                    | Report terminal completion and stop.                            |

The full verdict x next_action.kind dispatch table plus the pseudocode main loop with
pause-on-`$close` handling live in `references/autopilot-state-machine.md`. Read it when a
next_action shape is unfamiliar or when implementing the dispatch.

### Step 3 - Honor `$close`

Before every iteration, check whether the user has invoked `$close`. If yes, yield immediately and
do NOT resume without explicit user agreement. `$close` is the discussion boundary; `$autopilot`
must not fight it.

## When to pause for user input (MUST)

`$autopilot` MUST pause and hand control back to the user for:

- **Scope changes** - adding features, retargeting outcome, or altering OUTCOME.md intent.
- **Risk discovery** - architectural mismatch, security exposure, data-loss potential.
- **Money / billing / API credits / deployments** - anything that spends money or ships artifacts
  to users.
- **Destructive operations** - force-push, deleting production data, irreversible migrations,
  rewriting shared git history.
- **External systems not scoped in OUTCOME.md** - integrations, third-party auth, new vendors.
- **Any `blocked` verdict with a non-obvious resolution** - surface the status envelope and wait.

When in doubt, pause. Never invent user preferences to unblock a loop.

## Planning level

When dispatch hits `plan`, `$autopilot` must choose a level before invoking `$plan`:

- Default: `medium` for unambiguous missions.
- Escalate to `hard` when any pause trigger above applies (risk, security, data loss, money,
  deploy, destructive ops, architectural unknowns).
- Use `light` only when the mission is explicitly small and local (e.g. a documentation tweak in
  a single file, no architecture involved).

Record the effective level via:

```bash
codex1 plan choose-level --mission <id> --level <light|medium|hard>
```

Allow effective-level escalation if `choose-level` reports a higher required level than
requested.

## Stop conditions

Stop the loop when any of the following are true:

- `verdict: terminal_complete` in status.
- The user has invoked `$close` (pause, not terminate).
- The CLI returns an error code that autopilot cannot safely retry (anything non-`retryable`).
- A pause trigger above fires.

## Safety

`$autopilot` MUST NOT:

- Invent user preferences that change scope, risk, money, deploy, or destructive operations.
- Record review findings or verdicts; that belongs to `$review-loop`.
- Bypass mission-close review.
- Run `codex1 close complete` before `codex1 close check` reports `ready: true`.
- Modify STATE.json, PLAN.yaml, or OUTCOME.md directly. Always go through the CLI.
- Suppress Ralph stop pressure; respond to status, do not mute it.

## Resources

- `references/autopilot-state-machine.md` - complete verdict x next_action.kind dispatch table and
  the pseudocode main loop with pause-on-`$close` handling. Read before implementing dispatch
  logic or when a next_action shape is unfamiliar.
