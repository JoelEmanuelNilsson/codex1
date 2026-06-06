# Codex1 Workflow

Codex1 is a local artifact workflow, not native goal state.

## Flow

1. `$clarify` sharpens intent while questions are allowed.
2. `$create-prd` synthesizes known context into `PRD.md`.
3. `$plan` designs the lean executable route and `GOAL_BRIEF.md`.
4. The user asks Codex to create or refine a native `/goal` from `GOAL_BRIEF.md`.

`GOAL_BRIEF.md` is a rich native goal brief. It should not instruct Codex to read itself as the first execution step. It should shape the goal around a desired end state, specific evidence, preserved constraints, an iteration policy, tracking expectations, and blocked-report behavior. If the user needs an exact pasteable `/goal` prompt, use a compact suggested goal request or `GOAL_PROMPT.md`; do not shrink the whole brief just to satisfy a prompt limit.

## When Not To Plan

Do not use `$plan` for diagnosis, debugging, optimization research, benchmarking, code review, prompt writing, goal-prompt preparation, or any request where the user explicitly says not to use `$plan`. Use the relevant lane skill or direct workflow and write only the requested docs or prompt.

## Core Skills And Lane Skills

Core skills shape PRD-backed product missions: `$clarify`, `$create-prd`, and `$plan`.

Lane skills guide execution inside ready subplans: `$tdd`, `$diagnose`, and `$improve-codebase-architecture`. `$plan` assigns the lane; native `/goal` executes. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be fake ceremony.

Review helper skills guide evidence gathering without adding an execution lane. Use `$codex-review` during proof/QA or the review cycle when a second-model Codex review should inspect a local diff, branch, or commit. Its output is advisory evidence until main Codex verifies and triages it.

## Native Goal Boundary

Native `/goal` owns persistent objectives, continuation, pause/resume, usage accounting, and completion. Codex1 artifacts provide context and evidence. They do not manage native goals.

## Mechanical Commands

`codex1 setup` materializes repo-local skills and guidance. `codex1 init` creates the standard mission directory layout with path-safety checks. The CLI stops there: it does not judge readiness, write mission content, manage execution, or report completion.

## Proof/QA

Proof/QA is mission-scoped. It proves the PRD and ready subplans through tests, commands, Browser checks, screenshots, logs, manual checks, review evidence, or accepted-risk records. It is not a broad default dogfood audit of the whole app.
