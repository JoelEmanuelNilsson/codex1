# Round 3 decisions

Baseline audited: ccff0b2 (round-2 fixes). Round-3 audits committed at 89330e2. Fix commit: this commit.

Reviewer reports: `docs/audits/round-3/{cli-contract,e2e-walkthrough,handoff-cross-check,skills-audit,correctness-invariants,test-adequacy}.md`.

Cli-contract, correctness-invariants, and handoff-cross-check each reported 0 P0/P1/P2/P3. No cross-reviewer duplicates surfaced this round.

## FIX

- **e2e P1-1 · `outcome ratify` corrupts OUTCOME.md frontmatter** — `cli/outcome/ratify.rs::rewrite_status_to_ratified` now detects whether `body` starts with `'\n'` and only emits the trailing newline on the closing fence when the leading one is missing. Previously it unconditionally emitted `---` with no trailing newline on the assumption that `body` began with `\n`. That assumption held on the scaffolded template only on the *first* ratify (the template has the blank line), and never on hand-written OUTCOME.md files where the closing fence is directly followed by the first heading. Result: one successful ratify collapsed `---` and `# OUTCOME` onto a single line and permanently broke `split_frontmatter`. Tests added: `tests/outcome.rs::ratify_preserves_closing_fence_without_blank_body_prefix` (reproducer A — hand-written OUTCOME.md with no blank line after the closing fence; asserts the rewritten file still has a standalone `---` line and re-parses through `outcome check`), `tests/outcome.rs::ratify_is_file_level_idempotent_across_repeated_calls` (reproducer B — two successive ratifies on the scaffolded template; asserts the second ratify both succeeds and leaves the file parseable).
- **skills P1-1 · `review-loop` skill stalls on `mission_close_review_open` verdict** — updated `.codex/skills/review-loop/SKILL.md` step 1 of the mission-close workflow to accept both `ready_for_mission_close_review` (first round) and `mission_close_review_open` (re-entry after a dirty mission-close record where `cli/close/record_review.rs:205` flipped `close.review_state = Open`). The autopilot state-machine reference at `.codex/skills/autopilot/references/autopilot-state-machine.md:21` already routed `mission_close_review_open` to `$review-loop`, so no change to that file was needed — the bug was the skill refusing the handoff it was being handed. Doc-only edit.
- **test-adequacy P2-1 · `STATE_CORRUPT` path in `state::load`/`state::mutate` parse-failures has no direct test** — added `tests/foundation.rs::state_corrupt_envelope_on_invalid_state_json`: init a demo mission, overwrite `STATE.json` with garbage bytes, run `codex1 status --mission demo`, assert `ok:false`, `code:"STATE_CORRUPT"`, `retryable:false`, and that the message references "Failed to parse STATE.json". Prior coverage only hit the refuse-to-overwrite branch at `state/mod.rs:159` via `init_refuses_to_overwrite`; the `serde_json::from_str` branch at `state/mod.rs:84` had no direct integration trigger.
- **test-adequacy P2-2 · `full_mission_close_after_replan_reaches_terminal` doesn't assert CLOSEOUT.md** — extended the existing test in `tests/e2e_replan_trigger.rs` to assert `mission_dir.join("CLOSEOUT.md")` exists after `close complete` and that the rendered body contains the `CLOSEOUT` header, the mission id (`demo`), the tasks completed on the post-replan path (`T1`, `T4`), and the `terminal_at` timestamp. Closes the gap where the test's "full reproducer" doc-comment promised close-flow verification but a regression that skipped `atomic_write(CLOSEOUT.md)` while still bumping STATE would have passed.

## REJECT

- **test-adequacy P3-1 · `review_start_dry_run_enforces_expect_revision` omits `context.expected` / `context.actual`** — non-blocking P3; not in loop scope. The reviewer self-classified as defense-in-depth: the `RevisionConflict` envelope shape is independently pinned by `tests/foundation.rs::error_envelope_shape_is_stable_across_representative_codes` + unit test `core::error::tests::revision_conflict_envelope_shape_is_stable`, and eight sibling REVISION_CONFLICT integration tests already assert `context.expected`/`context.actual`.
- **test-adequacy P3-2 · only one of four TOCTOU `require_plan_locked`-in-closure paths has a concurrent test** — non-blocking P3; not in loop scope. The round-1 precedent (REJECT of correctness P2-2 "atomic_write crash-consistency test" with rationale that the concurrent-writer test exercises the module under real load) settled the bar at one concurrent test per mutation-module invariant; the reviewer explicitly flagged this P3 on that precedent.

## Totals

| Category | FIX | REJECT |
|----------|-----|--------|
| P0       |  0  |   0    |
| P1       |  2  |   0    |
| P2       |  2  |   0    |
| P3       |  0  |   2    |
