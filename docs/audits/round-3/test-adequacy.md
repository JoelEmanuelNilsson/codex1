# Round 3 — test-adequacy audit

Reviewer: test-adequacy (6/6). Lens: `crates/codex1/tests/*.rs` + `src/**` unit tests. Reading prior decisions first: all round-1 FIXes landed; round-2 FIXes landed including the P0 (replan.triggered clearing on relock). Rejected findings are not re-surfaced.

## Summary

- Test count verified: 179 integration `#[test]` across 16 files + 27 unit `#[test]` across `src/` = **206 tests** (matches prompt).
- Scrutiny of the five round-2 tests called out: 4-of-5 pass cleanly; 2 have minor partial-coverage gaps (not vacuous).
- **P0 slot is empty.** With round-2's P0 fix (`plan check` clears `state.replan.triggered`) now regressed end-to-end by `e2e_replan_trigger.rs::plan_check_after_replan_record_clears_triggered` (direct) and `…::full_mission_close_after_replan_reaches_terminal` (full reproducer through `close complete`), no fundamental invariant is untested.
- **One legitimate P2 gap** prior rounds missed: the `STATE_CORRUPT` code is only exercised via `init_write`'s "already exists" branch; the `state::load` parse-failure path (bad JSON in STATE.json) has no direct test.
- **One P3 defense-in-depth** gap: `review_start_dry_run_enforces_expect_revision` omits `context.expected` / `context.actual` assertions (covered elsewhere — strictly non-blocking).

### Scrutiny verdicts on round-2 tests

| Test | Scrutiny ask | Verdict |
|---|---|---|
| `e2e_replan_trigger.rs::plan_check_after_replan_record_clears_triggered` | Asserts `triggered == false` AND `triggered_reason == null`? | PASS — both asserted at lines 375–383. |
| `e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal` | Drives through to terminal; asserts `CLOSEOUT.md` + `terminal_at`? | PARTIAL — asserts `terminal_at` (line 580) but not `CLOSEOUT.md`. See P2-1. |
| `foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant` | Races two subprocess writers; genuine contention? | PASS — spawns two `std::thread` subprocesses racing `task start T2` vs `replan record --supersedes T2` without `--expect-revision` over 4 iterations; asserts the one-sided invariant `!(!plan.locked && task.status == in_progress)` that can only be satisfied by the in-closure `require_plan_locked` re-check (fs2 exclusive-lock serializes the mutates but the TOCTOU window lives between the pre-mutate shared-lock load and the closure; without the round-2 correctness P1-1 fix the race would be detectable). Not brittle serialization — the assertion would fail if the closure re-check were removed. Exercises only `task start` of the 4 protected paths; see P3-1. |
| `status.rs::task_next_unlocked_plan_emits_plan_kind` + `task_next_replan_triggered_emits_replan_kind` | Assert exact `kind` field? | PASS — both assert `data.next.kind` via `assert_eq!` (lines 540, 574). The replan case also pins `reason`. |
| `review.rs::review_start_dry_run_enforces_expect_revision` | Asserts `REVISION_CONFLICT` + `retryable: true` + `context.expected` + `context.actual`? | PARTIAL — asserts `code` + `retryable` (lines 558–559); does not assert `context.expected` / `context.actual`. See P3-1. |

## P0

None. No fundamental invariant is untested.

## P1

None.

## P2

### P2-1: `state::load` `STATE_CORRUPT` parse-failure path has no direct test

- **Citation.** `src/state/mod.rs:77` raises `CliError::StateCorrupt` when STATE.json is missing; `src/state/mod.rs:84` raises `CliError::StateCorrupt` when `serde_json::from_str` fails on the STATE.json body; `src/state/mod.rs:105,111` raise the same variant from inside `state::mutate`. `src/core/error.rs:93` maps the variant to the canonical `"STATE_CORRUPT"` code, listed in the frozen 18-code set (`iter4-cli-contract-audit.md` CC-3).
- **Evidence.** `Grep "STATE_CORRUPT" crates/codex1/tests` yields one hit: `foundation.rs:95` inside `init_refuses_to_overwrite`. That test exercises `init_write`'s refuse-to-overwrite branch (`src/state/mod.rs:159`), not `load` / `mutate`. Writing garbage into STATE.json and invoking any read-or-mutate command would exercise the parse-fail path at `state/mod.rs:84` — no test does this. Round-1 P1-4 was adjacent (Io/Yaml → PARSE_ERROR) but stopped at PLAN.yaml corruption, not STATE.json corruption; round-1 `foundation.rs::corrupt_plan_yaml_returns_plan_invalid_with_hint` routes through `PLAN_INVALID`, not `STATE_CORRUPT`. Prior rounds did not surface this specific gap.
- **Brief-item mapping.** Item 1 ("Every `CliError` variant constructed in production has a triggering test that asserts envelope `code`.") — STATE_CORRUPT is constructed in three distinct production sites (`state/mod.rs:77`, `:84`, `:105`/`:111`); only `:159` (a fourth site, reached via `init`) has a triggering integration test.
- **Suggested fix.** Add `tests/foundation.rs::corrupt_state_json_returns_state_corrupt`:
  1. `init_demo(&tmp, "demo")`.
  2. `fs::write(mission_dir.join("STATE.json"), "{ this is not json")?;`
  3. Run `codex1 status --mission demo` (or any read-only command going through `state::load`).
  4. Assert `json["ok"] == false`, `json["code"] == "STATE_CORRUPT"`, `json["message"].as_str().unwrap().contains("Failed to parse STATE.json")`, `!out.status.success()`.
  5. Optionally add a sibling test that removes STATE.json entirely and runs the same command to exercise the missing-file branch at `state/mod.rs:77`.

### P2-2: `full_mission_close_after_replan_reaches_terminal` asserts `terminal_at` but not `CLOSEOUT.md`

- **Citation.** `docs/cli-reference.md:415` contract: "`close complete` writes `CLOSEOUT.md` and marks the mission terminal." `src/cli/close/complete.rs` writes both `state.close.terminal_at` and `atomic_write(CLOSEOUT.md)`. The test's own doc-comment at `tests/e2e_replan_trigger.rs:386-390` claims it is a "full reproducer" driving through "`close complete`".
- **Evidence.** `tests/e2e_replan_trigger.rs:578-583` reads STATE.json and asserts `state["close"]["terminal_at"].is_string()`; there is no `fs::read_to_string(mission_dir.join("CLOSEOUT.md"))` check. The CLOSEOUT.md write is the final durable artifact of the close flow — a regression that skips the `atomic_write(CLOSEOUT.md)` call while still setting `terminal_at` via `state::mutate` would bump revision, stop the loop, and leave `terminal_at` set but no CLOSEOUT.md on disk. This test (labeled "full reproducer") would still pass. The round-2 P0 fix is narrow (clearing `replan.triggered`), so the test's value is regression protection, not close-flow verification — but the label oversells.
- **Brief-item mapping.** Item 4 ("task and review lifecycle transitions") — the mission-close → terminal transition is exercised by other tests (`e2e_full_mission.rs:460`, `close.rs:624` both assert CLOSEOUT.md content), so this is partial coverage on this specific test, not a system-wide gap. Calling it P2 because the test's "full reproducer" doc-comment implies completeness it doesn't deliver; a regression in the CLOSEOUT.md write on the post-replan path specifically would not be flagged by this test.
- **Suggested fix.** Add to `tests/e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal` after the existing `terminal_at` assertion:
  ```rust
  let closeout = fs::read_to_string(mission_dir.join("CLOSEOUT.md"))
      .expect("CLOSEOUT.md written after post-replan close complete");
  assert!(closeout.contains("CLOSEOUT"), "missing CLOSEOUT header: {closeout}");
  // The replaced (T4) and superseded (T2) tasks should both appear.
  for tid in ["T1", "T4"] {
      assert!(closeout.contains(tid), "CLOSEOUT.md missing {tid}: {closeout}");
  }
  ```
  No new test file; one additional block in the existing test.

## P3

### P3-1: `review_start_dry_run_enforces_expect_revision` omits `context.expected` / `context.actual`

- **Citation.** Scrutiny prompt specifically asks whether this round-2 test asserts `REVISION_CONFLICT` + `retryable: true` + `context.expected` + `context.actual`. `src/core/error.rs:148-151` pins `RevisionConflict.context = { expected, actual }`.
- **Evidence.** `tests/review.rs:558-559` asserts `err["code"] == "REVISION_CONFLICT"` and `err["retryable"] == true`; does not assert `err["context"]["expected"]` or `err["context"]["actual"]`. The envelope shape IS independently pinned by `tests/foundation.rs:221-226` (`error_envelope_shape_is_stable_across_representative_codes`) and the unit test `core::error::tests::revision_conflict_envelope_shape_is_stable`, plus eight sibling REVISION_CONFLICT integration tests that do assert `context.expected` / `context.actual` (`tests/task.rs:594-595`, `tests/task.rs:657-658`, `tests/loop_.rs:431-432`, `tests/close.rs:537-538`, `tests/close.rs:763-764`, `tests/outcome.rs:373-374`, `tests/plan_check.rs:612`, `tests/foundation.rs:223-225`). Defense-in-depth; strictly non-blocking.
- **Suggested fix.** One-liner add to `tests/review.rs:560`:
  ```rust
  assert_eq!(err["context"]["expected"], 999);
  assert!(err["context"]["actual"].is_u64());
  ```

### P3-2: Only one of four TOCTOU `require_plan_locked`-in-closure paths has a concurrent test

- **Citation.** The round-2 correctness P1-1 fix added `state::require_plan_locked(state)?;` inside the `state::mutate` closure for four commands: `src/cli/task/start.rs:111`, `src/cli/task/finish.rs:112`, `src/cli/review/start.rs:95`, `src/cli/review/record.rs:186`. Round-2's `concurrent_replan_and_task_start_preserves_plan_locked_invariant` races only `task start` vs `replan record`.
- **Evidence.** No concurrent test exists for `task finish`, `review start`, or `review record` vs `replan record`. The invariant (re-checking `plan.locked` under the exclusive lock) is exercised once per fix via non-concurrent integration tests (`tests/replan.rs::task_start_after_replan_record_refuses_with_plan_invalid` and its siblings for the other three paths), which catches the pre-load check but not the closure re-check. A regression that removes only one of the three untested closure re-checks would not be caught. Note: round-1 correctness P2-2 ("atomic_write crash-consistency test") was REJECTed with the rationale that the concurrent-writer test exercises the module under real load — the same precedent suggests one module-level concurrent test is accepted as sufficient. Flagging as P3 (not P2) because the prior precedent clearly settled the bar at one concurrent test for the `mutate` module.
- **Suggested fix.** Optional — parametrize `concurrent_replan_and_task_start_preserves_plan_locked_invariant` over the four protected commands via a helper that builds the racing pair, or add three sibling tests (`_and_task_finish_`, `_and_review_start_`, `_and_review_record_`). Each asserts the same one-sided invariant but for the corresponding command's state effect. Not required for loop gate.

---

## Totals

Counts below are per unique finding (no cross-reviewer dedupe needed at this layer — test-adequacy operates on the test surface only).

| Category | Count |
|----------|-------|
| P0       | 0     |
| P1       | 0     |
| P2       | 2     |
| P3       | 2     |
