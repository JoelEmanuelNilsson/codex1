# Goal Brief Format

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective for the whole mission.

It is a brief, not native goal state and not the final authority. The brief must not tell Codex to read `GOAL_BRIEF.md` as the first execution step.

## Goal Brief Must Include

- Purpose
- Suggested goal request
- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Explicit completion criteria
- If completion cannot be reached
- Closeout rules
- Prohibited actions

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Managing native goal state from Codex1.
- Treating `codex1 setup` status or `codex1 init` output as completion proof.
- Reading `GOAL_BRIEF.md` as the first step of the native goal.
