# Round 7 Heavy Review Findings

Date: 2026-04-21

This review repeated the heavy-review loop after the round-6 repair pass. It deployed 9 read-only `gpt-5.4-mini` reviewer lanes across path/contract safety, close/review/replan, waves/status/task-next, CLI/docs contract, E2E state invariants, skills/prompts, test adequacy, install UX, and round-6 regression review. Every reviewer was instructed to read `README.md` and all markdown files under `docs/` before judging code, including prior audit decisions and `docs/audits/round-6/*.md`, and to report only verified P0/P1/P2 findings.

Clean lanes:

- Close / review / replan state transitions.
- CLI docs / command-envelope contract.

## Summary

- P0: 0
- P1: 1
- P2: 13

## P1 Findings

### P1-1: Absolute-path and symlink containment branches are under-tested

Evidence:

- [crates/codex1/tests/foundation.rs](/Users/joel/codex1/crates/codex1/tests/foundation.rs:99) tests `init --mission ../escape`, but not absolute mission ids.
- [crates/codex1/tests/plan_check.rs](/Users/joel/codex1/crates/codex1/tests/plan_check.rs:284), [crates/codex1/tests/task.rs](/Users/joel/codex1/crates/codex1/tests/task.rs:404), and [crates/codex1/tests/review.rs](/Users/joel/codex1/crates/codex1/tests/review.rs:637) test `../../secret.md`, but not absolute spec paths or symlink escapes.
- [crates/codex1/src/core/paths.rs](/Users/joel/codex1/crates/codex1/src/core/paths.rs:101) contains separate branches for absolute paths, parent components, and post-canonicalization containment.

Impact:

Round 6 added important path-containment logic, but the highest-risk helper branches are not all covered. A regression that re-accepts absolute paths, or a symlink escape after lexical checks, could pass the current suite.

Suggested fix:

Add tests for absolute mission ids, absolute `tasks[].spec` paths, and a symlink escape through a mission-local path.

## P2 Findings

### P2-1: Stale-writer conflicts are masked by earlier domain gates in task/review mutators

Evidence:

- [crates/codex1/src/cli/task/start.rs](/Users/joel/codex1/crates/codex1/src/cli/task/start.rs:18) checks plan lock and task readiness before revision conflict handling.
- [crates/codex1/src/cli/task/finish.rs](/Users/joel/codex1/crates/codex1/src/cli/task/finish.rs:17) checks plan lock, task existence, proof path, and status before revision conflict handling.
- [crates/codex1/src/cli/review/start.rs](/Users/joel/codex1/crates/codex1/src/cli/review/start.rs:33) checks terminal / plan-lock preconditions before `check_expected_revision`.
- [crates/codex1/src/cli/review/record.rs](/Users/joel/codex1/crates/codex1/src/cli/review/record.rs:69) checks plan-lock and findings-file preconditions before revision conflict handling.
- Main thread reproduced `codex1 task start T1 --expect-revision 999` on an unlocked plan returning `PLAN_INVALID` instead of `REVISION_CONFLICT`.

Contract:

- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:74) defines strict-equality `--expect-revision` semantics for mutating commands.

Impact:

Callers using `--expect-revision` to detect stale writes can receive unrelated domain errors and make decisions against stale state.

Suggested fix:

Call `state::check_expected_revision(ctx.expect_revision, &state)?` immediately after `state::load` in task/review mutators, before plan-lock, proof, findings-file, terminal, dependency, or status preflight.

### P2-2: `plan check` can mask stale writers behind plan validation

Evidence:

- [crates/codex1/src/cli/plan/check.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/check.rs:31) reads and validates `PLAN.yaml` before loading state and checking `--expect-revision`.
- Reviewer and main thread reproduced malformed `PLAN.yaml` plus `--expect-revision 99` returning `PLAN_INVALID` instead of `REVISION_CONFLICT`.

Contract:

- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:74) says mutating commands use strict equality for stale-writer protection.

Impact:

Stale writers can be misdirected into plan repair instead of retrying against the current state revision.

Suggested fix:

Load state and enforce `--expect-revision` before plan parsing/validation in `plan check`.

### P2-3: `review record` can mask stale writers behind findings-file checks

Evidence:

- [crates/codex1/src/cli/review/record.rs](/Users/joel/codex1/crates/codex1/src/cli/review/record.rs:54) validates `--findings-file` before `state::check_expected_revision`.
- Reviewer reproduced a nonexistent findings file with `--expect-revision 99` returning `REVIEW_FINDINGS_BLOCK` instead of `REVISION_CONFLICT`.

Impact:

Same stale-writer contract violation as P2-1, with a concrete findings-file path.

Suggested fix:

Load/check state revision before findings-file existence checks.

### P2-4: `task finish` can mask stale writers behind proof checks

Evidence:

- [crates/codex1/src/cli/task/finish.rs](/Users/joel/codex1/crates/codex1/src/cli/task/finish.rs:16) validates proof existence before `state::check_expected_revision`.
- Reviewer reproduced a missing proof with `--expect-revision 99` returning `PROOF_MISSING` instead of `REVISION_CONFLICT`.

Impact:

Same stale-writer contract violation as P2-1, with a concrete proof path.

Suggested fix:

Load/check state revision before proof-file preflight.

### P2-5: Review readiness does not drop superseded targets

Evidence:

- [crates/codex1/src/cli/status/next_action.rs](/Users/joel/codex1/crates/codex1/src/cli/status/next_action.rs:79) `ready_reviews()` marks a review ready based on review-task readiness and clean-record absence.
- [crates/codex1/src/cli/task/lifecycle.rs](/Users/joel/codex1/crates/codex1/src/cli/task/lifecycle.rs:139) `next_ready_review()` only returns a review when it covers tasks in the awaiting-review set.
- Reviewer reproduced a review whose target was superseded: `status` reported `run_review`, while `task next` reported `run_task`.

Contract:

- `status` and `task next` are both public next-action surfaces, and prior docs/audits require them to agree on actionable work.

Impact:

Status can route `$review-loop` to a review whose target is no longer awaiting review.

Suggested fix:

Centralize review-readiness around a predicate that only surfaces reviews with at least one live `AwaitingReview` target.

### P2-6: Superseded tasks still skew wave derivation in status/task-next

Evidence:

- [crates/codex1/src/cli/status/next_action.rs](/Users/joel/codex1/crates/codex1/src/cli/status/next_action.rs:79) computes status waves over every task in `PLAN.yaml`.
- [crates/codex1/src/cli/task/lifecycle.rs](/Users/joel/codex1/crates/codex1/src/cli/task/lifecycle.rs:148) treats superseded dependencies as satisfied for task readiness.
- [crates/codex1/src/cli/plan/waves.rs](/Users/joel/codex1/crates/codex1/src/cli/plan/waves.rs:144) filters superseded tasks out of the live DAG first.
- Reviewer reproduced a mission where `plan waves` returned `DAG_MISSING_DEP`, while `status` / `task next` still surfaced a wave or review.

Impact:

The three public DAG/wave projections disagree after replan/supersession.

Suggested fix:

Derive all three surfaces from the same live-DAG rule. Either filter superseded ancestry consistently or fail consistently when a live task depends on a superseded task.

### P2-7: Clean close-review dry-run previews the stale dirty count

Evidence:

- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:95) dry-run clean emits `current_counter(current)`.
- The wet clean path resets `replan.consecutive_dirty_by_target["__mission_close__"]` to `0`.
- Reviewer reproduced a dirty counter seeded at 5: dry-run returned `consecutive_dirty: 5` instead of the predicted post-clean `0`.

Impact:

Dry-run does not preview the mutation it would perform.

Suggested fix:

Emit `consecutive_dirty: 0` for clean dry-run, matching the wet path.

### P2-8: Clarify skill omits required OUTCOME fields in its summary

Evidence:

- [.codex/skills/clarify/SKILL.md](/Users/joel/codex1/.codex/skills/clarify/SKILL.md:4) enumerates required fields but omits `status` and `definitions`.
- [docs/mission-anatomy.md](/Users/joel/codex1/docs/mission-anatomy.md:29) lists both as required.

Impact:

A future Codex thread following the skill literally can produce an OUTCOME that fails ratification or omits intended contract fields.

Suggested fix:

Add `status` and `definitions`, or replace the inline list with a pointer to the complete reference shape.

### P2-9: CLI reference example uses a nonexistent replan reason

Evidence:

- [docs/cli-reference.md](/Users/joel/codex1/docs/cli-reference.md:341) uses `six_consecutive_dirty`.
- [crates/codex1/src/cli/replan/triggers.rs](/Users/joel/codex1/crates/codex1/src/cli/replan/triggers.rs:13) allows `six_dirty`, `scope_change`, `architecture_shift`, `risk_discovered`, and `user_request`.

Impact:

Copying the documented example fails with `PLAN_INVALID` and can stall a replan handoff.

Suggested fix:

Change the example to `six_dirty` or another allowed reason.

### P2-10: Bare `status` hides ambiguous mission resolution

Evidence:

- [crates/codex1/src/cli/status/mod.rs](/Users/joel/codex1/crates/codex1/src/cli/status/mod.rs:31) converts every `MissionNotFound` into the no-mission fallback whenever `ctx.mission` is absent.
- Reviewer reproduced a directory with two valid `PLANS/*` missions where bare `codex1 status` returned `reason: no_mission` instead of surfacing the ambiguous-mission error.
- [docs/cli-contract-schemas.md](/Users/joel/codex1/docs/cli-contract-schemas.md:76) says discovery with more than one candidate errors unless `--mission` disambiguates.

Impact:

Ralph or a human can be told there is "no mission" when there are multiple missions and an explicit mission selection is needed.

Suggested fix:

Only use the graceful status fallback for true no-mission discovery failures. Propagate ambiguous-candidate `MISSION_NOT_FOUND` errors.

### P2-11: Verify target does not run the documented `/tmp` smoke

Evidence:

- [Makefile](/Users/joel/codex1/Makefile:35) `verify-installed` only checks `command -v`, `--help`, and `doctor`.
- [docs/install-verification.md](/Users/joel/codex1/docs/install-verification.md:29) says the critical verification is from `/tmp`, including `init` and `status`.

Impact:

`make verify-contract` can pass without proving the installed binary works outside the source tree for mission creation/status.

Suggested fix:

Fold the documented `/tmp` `init` / `status` smoke into `verify-installed` or add a separate target called by `verify-contract`.

### P2-12: `unknown_side_effects` blocker coverage is missing

Evidence:

- [crates/codex1/src/cli/status/next_action.rs](/Users/joel/codex1/crates/codex1/src/cli/status/next_action.rs:108) and [crates/codex1/src/cli/task/next.rs](/Users/joel/codex1/crates/codex1/src/cli/task/next.rs:173) treat `unknown_side_effects` as a blocker.
- Current tests cover `exclusive_resources`, but not `unknown_side_effects`, in status/task-next surfaces.

Impact:

The newly repaired parallel-safety branch can regress without test failure.

Suggested fix:

Add mirrored status/task-next tests for a ready wave containing `unknown_side_effects: true`.

### P2-13: Close-review staging failure path is under-tested

Evidence:

- [crates/codex1/src/cli/close/record_review.rs](/Users/joel/codex1/crates/codex1/src/cli/close/record_review.rs:177) stages mission-close dirty findings before state mutation.
- Current tests cover successful dirty findings and dry-run, but not a forced artifact-staging failure with state unchanged.

Impact:

Round-6's artifact-before-state invariant could regress without test failure.

Suggested fix:

Add a failure-path test that prevents writing to `reviews/` and asserts `STATE.json` and the mission-close dirty counter remain unchanged.
