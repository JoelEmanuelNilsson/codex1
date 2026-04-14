---
name: autopilot
description: End-to-end Codex1 mission workflow. Use when the user invokes $autopilot and wants the harness to keep advancing the same mission through clarify, planning, execution, review, and closure until it reaches an honest terminal or waiting verdict.
---

# Autopilot

Use this public skill as the thin composition surface over the same `clarify`,
`plan`, `execute`, and `review` contracts.

## Core Rule

`$autopilot` does not own separate semantics.

It must produce the same mission truth, gate outcomes, and verdict family that
a manual run would produce against the same repo and mission state.

Backend note:

- autopilot should compose the same internal command surface documented in
  `docs/runtime-backend.md`
- do not invent a separate hidden state path for autopilot-only behavior

## Workflow

1. Resolve the active mission from durable artifacts and the latest valid
   closeout.
2. Determine the next required branch from mission truth, not from tone or
   conversational vibes.
3. Invoke the same public workflow contracts as needed:
   - `clarify` until the mission is safely locked
   - `plan` until planning is complete enough for execution
   - `execute` on the packaged next target
   - `review` whenever a blocking review gate is reached
4. Use `internal-orchestration` for bounded subagents and `internal-replan`
   when contradictions require reopening.
5. Continue while the verdict is actionable non-terminal.
6. Yield only when the honest result is:
   - `needs_user`
   - `hard_blocked`
   - `complete`

## Continuation Rules

- Keep going when the next branch is known and Codex can continue autonomously.
- Treat `needs_user` as a durable waiting state, not as terminal completion.
- Do not rely on wording heuristics, tmux tricks, or hidden runtime glue as the
  real continuation authority.
- Prefer to create the PR when the mission bar is met and the repo context
  allows it.

## Must Not

- become a second hidden workflow engine
- stop early because the mission feels "probably done"
- bypass review or mission-close gates
- lose parity with the manual path

## Return Shape

Autopilot should leave one honest durable verdict and the visible artifacts that
explain why that verdict is true.
