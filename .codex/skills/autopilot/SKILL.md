---
name: autopilot
description: End-to-end Codex1 mission workflow. Use when the user invokes $autopilot and wants the harness to keep advancing the same mission through clarify, planning, execution, review-loop, and closure until it reaches an honest terminal or waiting verdict.
---

# Autopilot

Use this public skill as the thin composition surface over the same `clarify`,
`plan`, `execute`, and `review-loop` contracts.

## Core Rule

`$autopilot` does not own separate semantics.

It must produce the same mission truth, gate outcomes, and verdict family that
a manual run would produce against the same repo and mission state.

Backend note:

- begin a parent autopilot loop with `codex1 internal begin-loop-lease` using
  `mode = "autopilot_loop"` before relying on Ralph continuation
- autopilot should compose the same internal command surface documented in
  `docs/runtime-backend.md`
- do not invent a separate hidden state path for autopilot-only behavior

## Ralph Lease

`$autopilot` is the broadest parent/orchestrator loop. Acquire or refresh a
parent lease before autonomous continuation:

```json
{
  "mission_id": "<mission-id>",
  "mode": "autopilot_loop",
  "owner": "parent-autopilot",
  "reason": "User invoked $autopilot."
}
```

The lease covers the parent branch router only. Subagents spawned by autopilot
remain Ralph-exempt and may stop normally; the parent integrates their outputs.

## Autopilot Posture

- Autopilot is the branch router over the same public workflow contracts.
- It may keep acting autonomously when the next branch is machine-clear and the
  mission contract allows it.
- It must stay subordinate to package truth, review gates, contradictions, and
  durable waiting truth.

## Workflow

1. Resolve the active mission from durable artifacts and the latest valid
   closeout.
2. Determine the next required branch from mission truth, not from tone or
   conversational vibes.
3. Invoke the same public workflow contracts as needed:
   - `clarify` until the mission is safely locked
   - `plan` until planning is complete enough for execution
   - `execute` on the packaged next target
   - `review-loop` whenever a blocking review gate is reached
4. Use `internal-orchestration` for bounded subagents and `internal-replan`
   when contradictions require reopening.
5. Keep looping while the verdict is actionable non-terminal and the next branch
   is still machine-clear.
6. Yield only when the honest result is:
   - `needs_user`
   - `hard_blocked`
   - `complete`

## Branch Discipline

- If clarify truth is not lock-ready, the next branch is `clarify`.
- If clarify truth is lock-ready but durably waiting for manual `$plan`
  invocation, autopilot may consume that handoff and continue to `plan`.
- If planning truth or package truth is not complete enough for execution, the
  next branch is `plan`.
- If a passed package exists for the selected target, the next branch is
  `execute`.
- If a blocking review gate is open, failed, or stale, the next branch is
  `review-loop`.
- If the frontier is clean and the remaining owed gate is mission-close review,
  the next branch is `review-loop` for the mission-close bundle, not direct
  completion.
- If contradiction or replan truth says the current layer is no longer enough,
  the next branch is `internal-replan`, not continued execution.
- If the repo is durably waiting on the user, yield `needs_user` without
  terminalizing the mission.

## Continuation Rules

- Keep going when the next branch is known and Codex can continue autonomously.
- Treat `needs_user` as a durable waiting state, not as terminal completion.
- Do not rely on wording heuristics, tmux tricks, or hidden runtime glue as the
  real continuation authority.
- Preserve manual-path parity: the same mission truth should converge to the
  same artifact state, gate state, and verdict family whether the user drove
  the path manually or through autopilot.
- Prefer to create the PR when the mission bar is met and the repo context
  allows it.

## Must Not

- become a second hidden workflow engine
- stop early because the mission feels "probably done"
- bypass review or mission-close gates
- lose parity with the manual path
- treat `wait_agent`, subagent completion, or silence as proof that the mission
  is done

## Return Shape

Autopilot should leave one honest durable verdict and the visible artifacts that
explain why that verdict is true.
