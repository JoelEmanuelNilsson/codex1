# Codex1 Artifact Briefs

Codex1 artifacts should stay durable as code changes. Prefer behavior, interfaces, stable artifact names, and acceptance criteria over brittle paths or line numbers.

## PRD Quality

`PRD.md` should include problem statement, solution, extensive user stories, success criteria, boundaries, module sketch, implementation decisions, testing decisions, proof expectations, review expectations, and PR intent.

Success criteria should be observable, measurable outcomes that make the PRD satisfied. They are not implementation tasks or slice-level acceptance criteria.

Boundaries should separate `Always Preserve`, `Ask Before Changing`, and `Out Of Scope` work so execution agents know what must remain stable, what needs human approval, and what is intentionally excluded.

User stories should be numbered, behavior-focused, and broad enough that `$plan` can map slices back to them. Each story should describe one coherent behavior or outcome, not a vague bundle.

## Subplans As Agent Briefs

Ready subplans are contracts for future Codex work. Each ready subplan should include:

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

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective. The brief must include purpose, suggested goal request, mission path, primary artifacts to read, execution order, subplan selection, worker rules, editable scope, proof rules, review/triage rules, completion criteria, non-completion behavior, closeout, and prohibited actions.

Execution may not ask the user questions. If completion cannot be reached from artifacts, record non-completion evidence, accepted risks, or deferred work.

## Proof And Closeout

Proofs record commands, tests, Browser checks, screenshots, manual checks, failures, and accepted risks. Closeout is written only after auditing PRD satisfaction against proofs and reviews. Closeout does not complete native `/goal` by itself. Proof/QA proves the mission; it is not a broad default dogfood audit.
