# Goal Brief: Codex1 Lane Skills Alignment

## Purpose

Execute the Codex1 lane skills alignment mission end to end. Codex1 should become locally self-contained for its core workflow and execution lanes while staying light, adaptable, and native `/goal`-driven.

## Suggested Goal Request

Complete the whole Codex1 lane skills alignment mission from `.codex1/missions/codex1-lane-skills-alignment/`: implement repo-local lane skill setup, update plan/docs lane guidance, add behavioral tests, run proof/QA, and write closeout. Treat the PRD, PLAN, spec, ADR, and ready subplans as the mission contract.

## Mission Path

`.codex1/missions/codex1-lane-skills-alignment/`

## Primary Artifacts To Read

- `PRD.md`
- `PLAN.md`
- `ADRS/0001-repo-local-lane-skills.md`
- `SPECS/0001-codex1-lane-setup-contract.md`
- `SUBPLANS/ready/0001-setup-bundle-lane-skills.md`
- `SUBPLANS/ready/0002-plan-docs-execution-lanes.md`
- `SUBPLANS/ready/0003-tests-for-lane-skills-alignment.md`
- `SUBPLANS/ready/0004-proof-qa-and-closeout.md`

## Execution Order

1. Setup bundle lane skills.
2. Plan/docs execution lanes.
3. Tests for lane skills alignment.
4. Proof/QA and closeout.

## Subplan Selection Rules

Execute all ready AFK subplans unless a subplan is already completed, superseded, or proven not applicable. If a subplan cannot be completed from the artifacts, record non-completion evidence instead of asking the user.

## Worker/Subagent Rules

Use workers only when their file ownership can be cleanly separated. If workers are used, give them explicit ownership and proof expectations. Workers must not edit mission-level artifacts unless assigned.

## Editable Scope

Editable scope includes Codex1 setup implementation, templates, repo-local managed skill bodies, docs, tests, and mission artifacts for this mission. Do not edit global skill directories.

## Proof Recording Rules

Record proof in `PROOFS/`. Include commands run, summaries of outputs, failures, fixes, accepted risks, and any manual checks. Do not treat `codex1 inspect`, setup status, events, or receipts as completion proof by themselves.

## Review And Triage Rules

Review changes against `PRD.md`, the spec, and every ready subplan. If an issue is found, fix it when in scope. If it is out of scope, record it as deferred or accepted risk with a reason.

## Explicit Completion Criteria

- All required ready subplans are complete or explicitly triaged not applicable with evidence.
- Setup installs and reports the four lane skills as repo-local managed files.
- Lane skill content preserves original behavior closely with only small Codex1-local additions.
- Ready subplan guidance requires `Execution Lane` and lists allowed lanes.
- Docs explain core skills, lane skills, native `/goal`, and mission-scoped proof/QA.
- Behavioral tests cover the expanded setup bundle and lane guidance.
- Formatting, tests, and lints pass, or any non-passing result is recorded with accepted-risk reasoning.
- `PROOFS/` and `CLOSEOUT.md` exist and honestly summarize the mission result.

## If Completion Cannot Be Reached

Do not ask the user during goal execution. Record blockers, failed commands, partial work, deferred work, and the safest next step in mission artifacts.

## Closeout Rules

Write `CLOSEOUT.md` after proof exists. Closeout must compare completed work against PRD success, out-of-scope boundaries, remaining risks, and proof records.

## Prohibited Actions

- Do not create, inspect, mirror, or complete native goal state from Codex1.
- Do not treat Codex1 setup/status/events/receipts as mission completion proof.
- Do not create issue-tracker tickets.
- Do not read `GOAL_BRIEF.md` as the first execution step of native goal execution.
- Do not edit `/Users/joel/.agents/skills` or `/Users/joel/.codex/skills`.
- Do not revive old standalone dogfood as a default requirement.
