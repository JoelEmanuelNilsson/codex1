---
name: codex1
description: Repo-scoped Codex1 artifact workflow overview. Use as a router to the clarify, create-prd, and plan skills, and as a reminder of the native /goal boundary.
---

# Codex1

Codex1 is a tiny local setup and mission-scaffold helper. Its CLI owns repo-local skill materialization plus path-safe mission directory creation; Codex skills own PRDs, plans, goal briefs, specs, subplans, reviews, triage, proofs, and closeout.

Repo-local consumer docs installed by setup:

- `docs/agents/codex1-workflow.md`: the user-facing flow and native `/goal` boundary.
- `docs/agents/codex1-domain.md`: domain glossary and ADR consumption/production rules.
- `docs/agents/codex1-artifact-briefs.md`: PRD, subplan, goal brief, proof, review, and closeout quality bars.

Skill-local references installed by setup:

- `$clarify`: `ADR-FORMAT.md` and `CONTEXT-FORMAT.md`.
- `$create-prd`: `PRD-FORMAT.md`.
- `$plan`: `ADR-FORMAT.md`, `SUBPLAN-BRIEF.md`, and `GOAL-BRIEF-FORMAT.md`.
- `$tdd`: red-green-refactor guidance plus testing, mocking, interface, deep-module, and refactoring references.
- `$diagnose`: reproduce-first debugging guidance plus the HITL loop template.
- `$improve-codebase-architecture`: deep-module architecture guidance and interface references.
- `$prototype`: throwaway logic and UI prototype guidance.
- `$codex-review`: advisory Codex review closeout guidance plus a local helper script.
- `$handoff`: compact continuation notes for a fresh agent, saved outside the repo.

Preferred UX:

- Use `$clarify` to gather and preserve user intent while questions are still allowed.
- Use `$create-prd` to synthesize known context into `PRD.md`.
- Use `$plan` to design the mission and write `GOAL_BRIEF.md`.
- The user asks Codex to create or refine a native goal from the generated goal brief.

During execution, ready subplans may name an `Execution Lane`: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`. `$plan` assigns lanes; native `/goal` executes them.

Use `$codex-review` inside proof/QA or review cycles when a mission needs a second-model review pass. Review output is advisory evidence; Codex still owns triage, closeout judgment, and native `/goal` completion.

Use `$handoff` when a session should be compacted for another agent or future fresh context. Handoffs are temporary continuation notes, not Codex1 mission truth, proof, closeout, or native `/goal` state.

Native Codex `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion. Codex1 must not manage native goals.

Codex1 setup and init are mechanical helpers. They are not mission truth, task readiness, review pass/fail, proof sufficiency, close safety, or native goal state.
