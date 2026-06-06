# Goal Brief Format

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective for the whole mission.

It is a rich brief, not native goal state, not the final authority, and not necessarily the exact pasteable goal prompt. The brief must not tell Codex to read `GOAL_BRIEF.md` as the first execution step.

The suggested native goal is an exit contract. Keep the pasteable goal compact and criteria-focused; put longer context, rationale, and artifact inventories in the brief.

Do not force the whole brief under a character limit. If the user needs an exact pasteable `/goal` prompt, include a compact `Suggested Goal Request` or create `GOAL_PROMPT.md`. Apply the character limit to that prompt, not to the full brief.

## Goal Contract Pattern

Shape the suggested native goal around this contract:

```text
/goal <desired end state> verified by <specific evidence> while preserving <constraints>.
Between iterations, <how Codex chooses the next best action>.
If blocked or no valid paths remain, <what to report and what would unlock progress>.
```

The contract should name measurable outcomes, baseline or proxy measurements when available, validation commands or checks, realistic environment expectations, and the constraints that must stay intact.

When a number is impossible, use a precise observable substitute: parity target, checklist, eval, screenshot diff, review gate, log condition, or accepted-risk record. Avoid vague "good enough" or "pixel perfect" criteria unless another check defines them.

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
- Metrics, baselines or proxies, validation loop, and anti-gaming constraints
- Realistic environment: local, Browser, preview, staging, production-like data, external service, device, or explicit limitation
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
- Pre-execution stop-and-ask rules
- In-goal stop-and-report rules
- Completion report

Keep this prompt focused on executable behavior and evidence. Do not include setup history, artifact inventories, or background rationale unless they change execution.

The pasteable prompt should also prohibit obvious shortcuts for the mission: reducing scope to hit a metric, weakening tests, hiding failures, bypassing the intended workflow, or using a visual reference as the implementation itself.

## Execution Readiness

Before finalizing the brief, infer only the capabilities the mission actually needs. Check enough repo context to classify each relevant capability as `proven`, `safe during goal`, `needs user decision`, or `blocked`.

Common capability areas include source control, runtime, tests, external APIs, secrets, browser checks, deploy, data access, cost, security, time, visual comparison, and production-like environment access. Do not create a generic checklist; include only readiness facts that affect execution, stop rules, or proof.

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.
- Failed or superseded attempts were removed, reverted, or recorded as accepted risk.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Optional Goal Prompt

Use `GOAL_PROMPT.md` only when a separate copy source is clearer than embedding the prompt in `GOAL_BRIEF.md`. It should contain one pasteable native goal objective and no extra commentary. It must not instruct Codex to read itself; the user has already copied it.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Progress Handoff

For long-running goals, tell Codex where progress lives and when to update it. Use the lightest mechanism that preserves continuity:

- `notes.md` for decisions, measurements, blockers, and next steps.
- Meaningful commits or draft PR only when source-control progress matters.
- Progress artifact, screenshot, deployed preview, or status post only when a human or worker will consume it.

Do not require status machinery that has no consumer.

## Prohibited Actions

Always prohibit:

- Managing native goal state from Codex1.
- Treating `codex1 setup` status or `codex1 init` output as completion proof.
- Reading `GOAL_BRIEF.md` as the first step of the native goal.
- Declaring completion from setup success, clean intent, or partial progress without proof.
