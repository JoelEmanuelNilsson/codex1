---
name: codex1
description: Repo-scoped Codex1 artifact workflow overview. Use as a router to the clarify, create-prd, and plan skills, and as a reminder of the native /goal boundary.
---

# Codex1

Codex1 is a deterministic artifact helper for clarification context, PRD, PLAN, GOAL_BRIEF, SPEC, SUBPLAN, REVIEW, TRIAGE, PROOF, CLOSEOUT, receipts, and inventory inspection.

Repo-local consumer docs installed by setup:

- `docs/agents/codex1-workflow.md`: the user-facing flow and native `/goal` boundary.
- `docs/agents/codex1-domain.md`: domain glossary and ADR consumption/production rules.
- `docs/agents/codex1-artifact-briefs.md`: PRD, subplan, goal brief, proof, review, and closeout quality bars.

Skill-local references installed by setup:

- `$clarify`: `ADR-FORMAT.md` and `CONTEXT-FORMAT.md`.
- `$create-prd`: `PRD-FORMAT.md`.
- `$plan`: `ADR-FORMAT.md`, `SUBPLAN-BRIEF.md`, and `GOAL-BRIEF-FORMAT.md`.

Preferred UX:

- Use `$clarify` to gather and preserve user intent while questions are still allowed.
- Use `$create-prd` to synthesize known context into `PRD.md`.
- Use `$plan` to design the mission and write `GOAL_BRIEF.md`.
- The user asks Codex to create or refine a native goal from the generated goal brief.

Native Codex `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion. Codex1 must not create, mirror, inspect, or complete native goals.

Codex1 setup is mechanical repo guidance. It is not mission truth, task readiness, review pass/fail, proof sufficiency, close safety, or native goal state.
