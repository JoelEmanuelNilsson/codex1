# Codex1 Lane Setup Contract

## Purpose

Define the observable contract for adding Codex1 lane skills to repo-local setup without making Codex1 heavy or dependent on global skills.

## Managed Core Skills

Setup continues to install these core workflow skills:

- `.agents/skills/codex1/SKILL.md`
- `.agents/skills/clarify/SKILL.md`
- `.agents/skills/create-prd/SKILL.md`
- `.agents/skills/plan/SKILL.md`

## Managed Lane Skills

Setup adds these repo-local lane skills:

- `.agents/skills/tdd/SKILL.md`
- `.agents/skills/diagnose/SKILL.md`
- `.agents/skills/improve-codebase-architecture/SKILL.md`
- `.agents/skills/prototype/SKILL.md`

Each lane skill is a full local file. It must not depend on `/Users/joel/.agents/skills` or `/Users/joel/.codex/skills` existing.

## Skill Content Contract

- Preserve each lane's execution discipline as closely as practical.
- Add only small Codex1-local guidance where needed.
- Codex1-local guidance may mention mission artifacts, ready subplans, proof recording, closeout evidence, and the native `/goal` boundary.
- `SKILL.md` is the source of truth.
- `agents/openai.yaml` metadata is optional UI metadata and must not become semantically authoritative.

## Plan Contract

Every ready subplan must include `Execution Lane` with one allowed value:

- `tdd`
- `diagnose`
- `improve-codebase-architecture`
- `prototype`
- `proof-qa`
- `standard`

`$plan` assigns lanes. Native `/goal` executes the mission from artifacts. Codex1 does not create, inspect, mirror, or complete native goals.

## Proof/QA Contract

Codex1 proof/QA is mission-scoped. It verifies PRD and subplan acceptance criteria through tests, commands, Browser checks, screenshots, logs, manual checks, review evidence, or accepted-risk records. It is not the old broad dogfood audit.

## Setup Behavior

- Setup remains repo-local.
- Setup must not edit global skill directories.
- Install/update may add or update managed files.
- Removal of managed files requires explicit uninstall or future explicit prune behavior.
- Existing safety behavior for symlinks and writes outside the repo remains required.

## Observable Tests

Tests should verify behavior through CLI output, created files, setup status JSON, marker contents, and installed text. Avoid tests that only prove private helper internals.
