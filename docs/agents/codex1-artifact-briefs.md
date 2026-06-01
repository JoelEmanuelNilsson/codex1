# Codex1 Artifact Briefs

Codex1 artifacts should stay durable as code changes. Prefer behavior, interfaces, stable artifact names, and acceptance criteria over brittle paths or line numbers.

Completion scope default: assume the final finished product unless the user asks for staged delivery; exclusions go in boundaries; subplans are implementation slices, not product stages.

## Artifact Minimalism

Create the smallest artifact set that will actually guide execution. The usual planning spine is `PLAN.md`, `GOAL_BRIEF.md`, and ready subplans only when separate slices are useful.

Create `RESEARCH_PLAN.md`, `RESEARCH/`, `ADRS/`, `SPECS/`, and paused subplans only when they have a named future consumer. Empty or generic artifacts are context bloat.

## PRD Quality

`PRD.md` should include problem statement, solution, extensive user stories, success criteria, boundaries, module sketch, implementation decisions, testing decisions, proof expectations, review expectations, and PR intent.

Success criteria should be observable, measurable outcomes that make the PRD satisfied. They are not implementation tasks or slice-level acceptance criteria.

Boundaries should separate `Always Preserve`, `Ask Before Changing`, and `Out Of Scope` work so execution agents know what must remain stable, what needs human approval, and what is intentionally excluded.

User stories should be numbered, behavior-focused, and broad enough that `$plan` can map slices back to them. Each story should describe one coherent behavior or outcome, not a vague bundle.

## Subplans As Agent Briefs

Ready subplans are contracts for future Codex work. Keep them compact enough to be read and acted on. Each ready subplan should include:

- slice type: `AFK` or already-resolved `HITL`
- execution lane: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`
- current behavior or repo state
- desired behavior
- key interfaces or stable contracts
- in-scope and out-of-scope work
- dependencies and blocked-by relationships
- acceptance criteria
- expected proof
- exit criteria

Write subplans as tracer bullets: thin vertical slices that deliver a complete, independently verifiable path through the system.

`standard` is the escape hatch for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial. `$plan` assigns lanes; native `/goal` executes from them.

## Goal Brief

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective. It is rich mission context, not automatically the exact pasteable prompt. The brief must include purpose, suggested goal request, mission path, primary artifacts to read, execution order, subplan selection, worker rules, editable scope, proof rules, review/triage rules, completion criteria, non-completion behavior, closeout, and prohibited actions.

Shape the suggested goal as: desired end state, verified by specific evidence, while preserving named constraints. Include how Codex should choose the next best action between continuations, what to track in `notes.md` for long-running work, and what to report if blocked or no valid path remains.

When useful, include mission-specific metrics, baselines or proxies, validation commands, and readiness facts. Readiness is not a generic checklist; classify only relevant capabilities as `proven`, `safe during goal`, `needs user decision`, or `blocked`.

If the user needs the exact `/goal` text under a character limit, write a compact suggested goal request or optional `GOAL_PROMPT.md`. Apply the limit to the pasteable prompt, not to the full brief.

Execution may not ask the user questions. If completion cannot be reached from artifacts, record non-completion evidence, accepted risks, or deferred work.

## Proof And Closeout

Proofs record commands, tests, Browser checks, screenshots, manual checks, failures, and accepted risks. Closeout is written only after auditing PRD satisfaction against proofs and reviews. Closeout does not complete native `/goal` by itself. Proof/QA proves the mission; it is not a broad default dogfood audit.
