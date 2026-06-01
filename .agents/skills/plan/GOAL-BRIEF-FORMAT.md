# Goal Brief Format

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective for the whole mission.

It is a rich brief, not native goal state, not the final authority, and not necessarily the exact pasteable goal prompt. The brief must not tell Codex to read `GOAL_BRIEF.md` as the first execution step.

Do not force the whole brief under a character limit. If the user needs an exact pasteable `/goal` prompt, include a compact `Suggested Goal Request` or create `GOAL_PROMPT.md`. Apply the character limit to that prompt, not to the full brief.

## Goal Contract Pattern

Shape the suggested native goal around this contract:

```text
/goal <desired end state> verified by <specific evidence> while preserving <constraints>.
Between iterations, <how Codex chooses the next best action>.
If blocked or no valid paths remain, <what to report and what would unlock progress>.
```

The contract should name measurable outcomes, baseline or proxy measurements when available, validation commands or checks, and the constraints that must stay intact.

If the user will paste text after an already-entered `/goal`, omit the `/goal` prefix from the copy source.

## Goal Brief Must Include

- Purpose
- Suggested goal request, compact when it is meant to be pasted after `/goal`
- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Metrics, baselines or proxies, and validation loop
- Iteration policy: how Codex chooses the next best action between continuations
- Tracking rule: for long-running work, maintain `notes.md` with decisions, measurements, blockers, and next steps
- Explicit completion criteria
- If completion cannot be reached
- Stop and ask rules before execution; stop and report rules during execution
- Closeout rules
- Prohibited actions

## Pasteable Goal Structure

When `Suggested Goal Request` or `GOAL_PROMPT.md` is meant to be pasted, include these sections or their compact equivalents:

- Objective
- Context
- Success criteria
- Feedback loop
- Tracking
- Constraints
- Stop and ask rules
- Completion report

Keep this prompt focused on executable behavior and evidence. Do not include setup history, artifact inventories, or background rationale unless they change execution.

## Execution Readiness

Before finalizing the brief, infer only the capabilities the mission actually needs. Check enough repo context to classify each relevant capability as `proven`, `safe during goal`, `needs user decision`, or `blocked`.

Common capability areas include source control, runtime, tests, external APIs, secrets, browser checks, deploy, data access, cost, security, and time. Do not create a generic checklist; include only readiness facts that affect execution, stop rules, or proof.

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Optional Goal Prompt

Use `GOAL_PROMPT.md` only when a separate copy source is clearer than embedding the prompt in `GOAL_BRIEF.md`. It should contain one pasteable native goal objective and no extra commentary. It must not instruct Codex to read itself; the user has already copied it.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Managing native goal state from Codex1.
- Treating `codex1 setup` status or `codex1 init` output as completion proof.
- Reading `GOAL_BRIEF.md` as the first step of the native goal.
