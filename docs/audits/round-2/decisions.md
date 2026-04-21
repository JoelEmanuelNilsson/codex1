# Round 2 decisions

Baseline audited: 05fcae3 (round-2 audits committed at 1e95dd6).

Reviewer reports: `docs/audits/round-2/{cli-contract,e2e-walkthrough,handoff-cross-check,skills-audit,correctness-invariants,test-adequacy}.md`.

## FIX

- **e2e P0-1 · `state.replan.triggered` never cleared on relock** — extended `cli/plan/check.rs::run` mutation closure to set `state.replan.triggered = false; state.replan.triggered_reason = None;` alongside the existing lock/hash/phase mutations. Without this, any mission that entered replan was bricked: `status.verdict` stayed `blocked`, `close check`/`close complete`/`close record-review` refused to advance, autopilot looped on `kind: replan` forever. Tests: `tests/e2e_replan_trigger.rs::plan_check_after_replan_record_clears_triggered` (narrow), `tests/e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal` (full reproducer: replan → plan check → new work → mission-close review → close complete reaching terminal).
- **correctness P1-1 · `require_plan_locked` TOCTOU between pre-mutate shared-lock load and mutate closure** — added a re-check of `state::require_plan_locked(state)?;` as the first line inside the `state::mutate` closure for each of `task/start.rs`, `task/finish.rs`, `review/start.rs`, and `review/record.rs`. On `review/record.rs` the check is skipped when `state.close.terminal_at.is_some()` so the existing terminal-contamination classification still wins precedence (round-1 pattern). The pre-load check is kept as a fast-fail optimization. Test: `tests/foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant` — races `task start T2` against `replan record --supersedes T2 --reason six_dirty` across 4 iterations without `--expect-revision`; asserts the final on-disk shape is never `!plan.locked && tasks.T2.status == "in_progress"`.
- **skills P1-1 · `plan/references/dag-quality.md:46` missing `--reason` on `replan record`** — updated the line to `codex1 replan record --reason <code> --supersedes <id>` with a pointer to `crates/codex1/src/cli/replan/triggers.rs::ALLOWED_REASONS`. Doc-only fix; existing `tests/replan.rs::record_rejects_unknown_reason` covers the CLI contract.
- **e2e P2-1 · `codex1 task next` ignored `plan.locked` and `replan.triggered`** — short-circuited `cli/task/next.rs::run` on `!state.plan.locked` (emit `{kind:"plan",hint:"Draft and lock PLAN.yaml."}`) and on `state.replan.triggered` (emit `{kind:"replan",reason:…}`). Mirrors the round-1 P2-2 fix in `cli/status/project.rs::build`. Tests: `tests/status.rs::task_next_unlocked_plan_emits_plan_kind` and `tests/status.rs::task_next_replan_triggered_emits_replan_kind`.
- **correctness P2-1 · `review start` dry-run skipped `check_expected_revision`** — added `state::check_expected_revision(ctx.expect_revision, &state)?;` to the dry-run branch in `cli/review/start.rs::run` so every `--expect-revision` honoring path uses strict equality consistently with the round-1 short-circuit sweep (task/start idempotent+dry-run, task/finish dry-run, close/complete, outcome/ratify, loop, plan/check, plan/choose-level, review/record). Test: `tests/review.rs::review_start_dry_run_enforces_expect_revision`.
- **skills P2-1 · SKILL.md prose used nested STATE paths for the flat status envelope** — updated `.codex/skills/plan/SKILL.md:20` (`outcome.ratified` → `outcome_ratified`) and `.codex/skills/execute/SKILL.md:16` (`plan.locked` → `plan_locked`) to match the actual `codex1 --json status` data envelope at `cli/status/project.rs:72-73`. Doc-only fix.

## REJECT

- **handoff-cross-check P3 F1 · `ParsedPlan` lacks `#[serde(deny_unknown_fields)]`** — non-blocking P3; not in loop scope. The anti-goal ("waves are derived, not stored") is already honored at the storage layer (no `waves` field in `ParsedPlan` or `MissionState`); this is UX-level polish the reviewer itself classified as "Not loop-scope per decisions.md rule 5".
- **correctness P3-1 · `src/cli/plan/dag.rs:51` `.expect("indegree entry")` lacks invariant comment** — non-blocking P3; not in loop scope. Same category as round-1 correctness P3-4 (`.expect` comments), which was also rejected.
- **correctness P3-2 · `replan record` / `plan scaffold` dry-run open-code `expected != state.revision`** — non-blocking P3; not in loop scope. The invariant is equivalent to the helper call; style-only drift.
- **correctness P3-3 · `close/record_review.rs` dirty path mutates STATE before findings-file write** — non-blocking P3; not in loop scope. The behavior is intentional and already documented at the site; reviewer asked only for a cross-reference comment.
- **test-adequacy P3-1 · table-driven `every_variant_envelope_round_trips` test** — non-blocking P3; not in loop scope. Existing integration coverage already hits every variant's `code`; round-1 added unit tests for the reserved variants.
- **test-adequacy P3-2 · proptest-style Verdict/status agreement test** — non-blocking P3; reviewer concurred this was rejected in round 1 and surfaced only to close the loop.

## Totals

Counts below are per unique finding after cross-reviewer dedupe. No new cross-reviewer duplicates: cli-contract, handoff-cross-check, and test-adequacy each reported 0 new P0/P1/P2; e2e, correctness, and skills each reported distinct findings.

| Category | FIX | REJECT |
|----------|-----|--------|
| P0       |  1  |   0    |
| P1       |  2  |   0    |
| P2       |  3  |   0    |
| P3 (non-blocking, out of scope) | 0 | 6 |
