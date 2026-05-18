# Codex1 Workflow

Codex1 is a local artifact workflow, not an issue tracker and not native goal state.

## Flow

1. `$clarify` sharpens intent while questions are allowed.
2. `$create-prd` synthesizes known context into `PRD.md`.
3. `$plan` designs research, specs, ADRs, vertical subplans, and `GOAL_BRIEF.md`.
4. The user asks Codex to create or refine a native `/goal` from `GOAL_BRIEF.md`.

`GOAL_BRIEF.md` is a native goal brief. It should not instruct Codex to read itself as the first execution step.

## Core Skills And Lane Skills

Core skills shape the mission: `$codex1`, `$clarify`, `$create-prd`, and `$plan`.

Lane skills guide execution inside ready subplans: `$tdd`, `$diagnose`, `$improve-codebase-architecture`, and `$prototype`. `$plan` assigns the lane; native `/goal` executes. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be fake ceremony.

## No Issue Tracker

Codex1 does not publish PRDs, issues, or plans to GitHub Issues, Linear, Jira, GitLab, or any other tracker. Durable work lives in `.codex1/missions/<id>/`.

## Native Goal Boundary

Native `/goal` owns persistent objectives, continuation, pause/resume, usage accounting, and completion. Codex1 artifacts provide context and evidence. They do not create, mirror, inspect, or complete native goals.

## Mechanical Commands

`codex1 setup`, `codex1 inspect`, events, and receipts are mechanical helpers. They are not proof of readiness, review cleanliness, PRD satisfaction, closeout, or native goal state.

## Proof/QA

Proof/QA is mission-scoped. It proves the PRD and ready subplans through tests, commands, Browser checks, screenshots, logs, manual checks, review evidence, or accepted-risk records. It is not a broad default dogfood audit of the whole app.
