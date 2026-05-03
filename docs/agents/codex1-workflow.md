# Codex1 Workflow

Codex1 is a local artifact workflow, not an issue tracker and not native goal state.

## Flow

1. `$clarify` sharpens intent while questions are allowed.
2. `$create-prd` synthesizes known context into `PRD.md`.
3. `$plan` designs research, specs, ADRs, vertical subplans, and `EXECUTION_PROMPT.md`.
4. The user manually starts a new Codex CLI session, types `/goal`, and pastes the objective from `EXECUTION_PROMPT.md`.

`EXECUTION_PROMPT.md` is a copy source. It should not instruct Codex to read itself.

## No Issue Tracker

Codex1 does not publish PRDs, issues, or plans to GitHub Issues, Linear, Jira, GitLab, or any other tracker. Durable work lives in `.codex1/missions/<id>/`.

## Native Goal Boundary

Native `/goal` owns persistent objectives, continuation, pause/resume, usage accounting, and completion. Codex1 artifacts provide context and evidence. They do not create, mirror, inspect, or complete native goals.

## Mechanical Commands

`codex1 setup`, `codex1 inspect`, events, and receipts are mechanical helpers. They are not proof of readiness, review cleanliness, PRD satisfaction, closeout, or native goal state.
