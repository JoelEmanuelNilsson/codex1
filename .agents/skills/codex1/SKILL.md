---
name: codex1
description: Repo-scoped Codex1 artifact workflow overview. Use as a router to the clarify, create-prd, and plan skills, and as a reminder of the native /goal boundary.
---

# Codex1

Codex1 is a deterministic artifact helper for clarification context, PRD, PLAN, EXECUTION_PROMPT, SPEC, SUBPLAN, REVIEW, TRIAGE, PROOF, CLOSEOUT, receipts, and inventory inspection.

Repo-local consumer docs installed by setup:

- `docs/agents/codex1-workflow.md`: the user-facing flow and native `/goal` boundary.
- `docs/agents/codex1-domain.md`: domain glossary and ADR consumption/production rules.
- `docs/agents/codex1-artifact-briefs.md`: PRD, subplan, execution prompt, proof, review, and closeout quality bars.

Preferred UX:

- Use `$clarify` to gather and preserve user intent while questions are still allowed.
- Use `$create-prd` to synthesize known context into `PRD.md`.
- Use `$plan` to design the mission and write `EXECUTION_PROMPT.md`.
- The user manually starts a new Codex CLI session, types `/goal`, and pastes the generated objective.

Native Codex `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion. Codex1 must not create, mirror, inspect, or complete native goals.

Codex1 setup is mechanical repo guidance. It is not mission truth, task readiness, review pass/fail, proof sufficiency, close safety, or native goal state.
