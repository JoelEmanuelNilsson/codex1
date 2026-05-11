---
codex1_template: goal-brief
template_version: 1
---

# Codex1 Workflow Redesign Goal Brief

<!-- codex1-section: purpose -->
## Purpose

Use this brief to create or refine a native Codex mission goal for `.codex1/missions/codex1-workflow-redesign`. The native goal should execute the whole mission plan, not one isolated slice. Codex1 artifacts provide context and evidence; native `/goal` owns continuation and completion.

<!-- codex1-section: suggested_goal_request -->
## Suggested Goal Request

Ask Codex to create or set a native goal from this brief:

```text
Execute the Codex1 mission at `.codex1/missions/codex1-workflow-redesign` end to end.

Use `PRD.md` as the outcome contract, `PLAN.md` as the execution strategy, `ADRS/` as durable mission decisions, `SPECS/` as implementation contracts, and `SUBPLANS/ready/` as executable vertical slices. Complete ready subplans in the order recommended by `PLAN.md`, record proof after each completed slice, triage review findings when applicable, and write closeout only after auditing PRD satisfaction against evidence.

Do not create issue-tracker tickets. Do not implement native goals inside Codex1. Do not treat Codex1 inspect, setup status, events, receipts, folder placement, proof artifacts, review artifacts, or closeout as native goal state. If completion cannot be reached from the artifacts, record the blocker, accepted risk, or deferred work instead of inventing scope.
```

<!-- codex1-section: mission_path -->
## Mission Path

`.codex1/missions/codex1-workflow-redesign`

<!-- codex1-section: primary_artifacts -->
## Primary Artifacts To Read

- `PRD.md`
- `PLAN.md`
- `ADRS/0001-current-only-artifact-catalog.md`
- `SPECS/0001-artifact-catalog-and-goal-brief-cli-contract.md`
- `SPECS/0001-managed-workflow-guidance-contract.md`
- `SPECS/0001-legacy-terminology-and-anti-oracle-verification-contract.md`
- `SUBPLANS/ready/0001-core-goal-brief-artifact-catalog.md`
- `SUBPLANS/ready/0001-managed-workflow-guidance-goal-brief.md`
- `SUBPLANS/ready/0001-regression-proof-and-closeout.md`

<!-- codex1-section: execution_order -->
## Execution Order

- Complete the core goal-brief artifact catalog slice first.
- Complete the managed workflow guidance slice second.
- Complete the regression proof and closeout slice last.
- Keep the repo working at the end of each slice.
- Record non-completion evidence instead of asking execution-phase planning questions.

<!-- codex1-section: subplan_selection -->
## Subplan Selection

- Treat `SUBPLANS/ready/` as the executable set.
- Execute ready subplans in the order recommended by `PLAN.md`.
- Do not execute HITL or paused work unless a later artifact explicitly resolves the human decision.
- If a ready subplan becomes obsolete, record why and move or supersede it through Codex1 artifacts instead of silently skipping it.

<!-- codex1-section: worker_rules -->
## Worker And Subagent Rules

- Workers may be used for bounded slices with explicit ownership.
- Workers should not edit mission-level artifacts unless assigned.
- Do not give two workers overlapping write ownership.
- Review worker changes before recording proof.

<!-- codex1-section: editable_scope -->
## Editable Scope

- Codex1 CLI code, tests, docs, managed skill files, setup-managed bodies, and mission artifacts needed by this mission.
- Do not edit unrelated project behavior.
- Do not publish to issue trackers or create PRs unless explicitly requested later.

<!-- codex1-section: proof_rules -->
## Proof Recording

- Write proof artifacts in `PROOFS/` after completed implementation slices.
- Proof should include commands run, tests run, changed areas, failures, accepted risks, and evidence links.
- Use external behavior tests through the CLI where possible.
- Use search evidence to classify remaining legacy terminology.

<!-- codex1-section: review_triage_rules -->
## Review And Triage

- Review should focus on stale current terminology, accidental compatibility duplicates, anti-oracle regressions, setup bundle drift, and whether goal brief language sounds like Codex1-owned goal state.
- Triage accepted, rejected, duplicate, stale, or deferred findings in `TRIAGE/` when review occurs.
- Review artifacts are opinions; triage is Codex judgment; neither is native goal state.

<!-- codex1-section: completion_criteria -->
## Completion Criteria

- Required ready subplans are complete or explicitly triaged not applicable.
- PRD success criteria are satisfied or explicitly deferred with reason.
- Required proof commands have run or failures are recorded with accepted-risk rationale.
- No current CLI, setup-managed output, README, or workflow docs present `EXECUTION_PROMPT.md` or `execution-prompt` as the current artifact.
- Remaining legacy mentions are clearly marked as legacy reading guidance only.
- `CLOSEOUT.md` audits PRD satisfaction against proofs and reviews.

<!-- codex1-section: non_completion_behavior -->
## If Completion Cannot Be Reached

- Record the blocker in `PROOFS/` or `CLOSEOUT.md`.
- Record accepted risks and deferred work explicitly.
- Do not invent scope to force completion.
- Do not ask the user during native-goal execution unless a HITL artifact explicitly requires user input.

<!-- codex1-section: closeout_rules -->
## Closeout

- Write `CLOSEOUT.md` only after auditing PRD success criteria against evidence.
- Closeout should summarize completed, superseded, paused, deferred, and risky work.
- Closeout is evidence for Codex judgment; it does not complete the native goal by itself.

<!-- codex1-section: prohibited_actions -->
## What Not To Do

- Do not create, inspect, mirror, or complete native goal state from Codex1.
- Do not add execution-prompt as a current CLI alias.
- Do not generate both `EXECUTION_PROMPT.md` and `GOAL_BRIEF.md`.
- Do not create a standalone clarification artifact.
- Do not publish to an issue tracker.
- Do not implement Browser-native dogfood in this mission.
- Do not treat `codex1 inspect`, setup status, events, receipts, or folder placement as proof of completion.
