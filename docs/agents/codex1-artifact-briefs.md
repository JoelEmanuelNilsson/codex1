# Codex1 Artifact Briefs

Codex1 artifacts should stay durable as code changes. Prefer behavior, interfaces, stable artifact names, and acceptance criteria over brittle paths or line numbers.

## PRD Quality

`PRD.md` should include problem statement, solution, extensive user stories, success criteria, module sketch, implementation decisions, testing decisions, out-of-scope work, proof expectations, review expectations, and PR intent.

User stories should be numbered and broad enough that `$plan` can map slices back to them.

## Subplans As Agent Briefs

Ready subplans are contracts for future Codex work. Each ready subplan should include:

- slice type: `AFK` or already-resolved `HITL`
- current behavior or repo state
- desired behavior
- key interfaces or stable contracts
- in-scope and out-of-scope work
- dependencies and blocked-by relationships
- acceptance criteria
- expected proof
- exit criteria

Write subplans as tracer bullets: thin vertical slices that deliver a complete, independently verifiable path through the system.

## Goal Brief

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective. The brief must include purpose, suggested goal request, mission path, primary artifacts to read, execution order, subplan selection, worker rules, editable scope, proof rules, review/triage rules, completion criteria, non-completion behavior, closeout, and prohibited actions.

Execution may not ask the user questions. If completion cannot be reached from artifacts, record non-completion evidence, accepted risks, or deferred work.

## Proof And Closeout

Proofs record commands, tests, screenshots, manual checks, failures, and accepted risks. Closeout is written only after auditing PRD satisfaction against proofs and reviews. Closeout does not complete native `/goal` by itself.
