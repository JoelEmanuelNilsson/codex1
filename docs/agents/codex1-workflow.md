# Codex1 Workflow

Codex1 is a local artifact workflow, not native goal state.

## Flow

1. `$clarify` sharpens intent while questions are allowed.
2. `$create-prd` synthesizes known context into `PRD.md`.
3. Codex executes directly from the PRD, or the user asks Codex to create or refine a native `/goal` when persistence is useful.

If the user needs exact pasteable `/goal` text, write a compact goal request or `GOAL_PROMPT.md`. Do not make Codex1 own native goal state.

## Core Skills And Lane Skills

Core skills shape PRD-backed product missions: `$clarify` and `$create-prd`.

Lane skills guide execution when their discipline fits: `$tdd`, `$diagnose`, and `$improve-codebase-architecture`. Use direct execution for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be fake ceremony.

Review helper skills guide evidence gathering without adding an execution lane. Use `$codex-review` during proof/QA or the review cycle when a second-model Codex review should inspect a local diff, branch, or commit. Its output is advisory evidence until main Codex verifies and triages it.

## Native Goal Boundary

Native `/goal` owns persistent objectives, continuation, pause/resume, usage accounting, and completion. Codex1 artifacts provide context and evidence. They do not manage native goals.

## Mechanical Commands

`codex1 setup` materializes repo-local skills and guidance. `codex1 init` creates the standard mission directory layout with path-safety checks. The CLI stops there: it does not judge readiness, write mission content, manage execution, or report completion.

## Proof/QA

Proof/QA is mission-scoped. It proves the PRD and ready subplans through tests, commands, Browser checks, screenshots, logs, manual checks, review evidence, or accepted-risk records. It is not a broad default dogfood audit of the whole app.
