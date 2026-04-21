# Round 4 — test-adequacy audit

Reviewer: test-adequacy (6/6). Lens: `crates/codex1/tests/*.rs` + `src/**` unit tests. HEAD: `703a171`. Prior decisions consumed: round-1, round-2, round-3. Rejected findings not re-surfaced.

## Summary

- **Test count verified**: 182 integration `#[test]` (16 files) + 27 unit `#[test]` (5 files) = **209** (matches prompt: 206 + 3 round-3 additions).
- **Round-3 scrutiny: 4-of-4 pass.** None of the four round-3 tests called out by the prompt are vacuous. Detail below.
- **One new P2 gap** prior rounds did not surface: `CliError::ReviewFindingsBlock` is constructed in production at `src/cli/review/record.rs:57` but has no triggering integration test that asserts the envelope `code`. The single `REVIEW_FINDINGS_BLOCK` string assertion in the test suite (`tests/close.rs:304`) originates from an unrelated production site (`src/cli/close/check.rs:127` blocker list).
- **No P0 / P1 findings.** Every `CliError` variant except `ReviewFindingsBlock` has either an integration-level envelope assertion or a unit-level envelope assertion (reserved variants, per round-1 P1-1/P1-2). All eight `Verdict` arms are covered by `src/state/readiness.rs::tests`. Task/review lifecycle, dirty-counter edge cases, stop-hook contract, and concurrency invariants are all covered.

### Round-3 test scrutiny verdicts

| Test | Scrutiny ask | Verdict |
|---|---|---|
| `tests/outcome.rs::ratify_preserves_closing_fence_without_blank_body_prefix` | Does it assert OUTCOME.md parses after ratify, or just that ratify succeeded? | **PASS — non-vacuous.** Asserts the rewritten file does not contain `"---# OUTCOME"` (collapsed fence), asserts a standalone `---` fence line exists, asserts the flipped `status: ratified` substring, then runs `outcome check` and asserts `ok:true`/`ratifiable:true`. `outcome check` re-parses OUTCOME.md via `validate_outcome` (`src/cli/outcome/validate.rs:25`), so the assertion is grounded on a fresh parse, not command success. |
| `tests/outcome.rs::ratify_is_file_level_idempotent_across_repeated_calls` | Does the second ratify actually re-read and re-parse OUTCOME.md, or does it short-circuit on `state.outcome.ratified == true`? | **PASS — non-vacuous.** `src/cli/outcome/ratify.rs::run` does NOT short-circuit on `state.outcome.ratified`. It unconditionally (1) calls `validate_outcome(&paths.outcome())` at line 19 which re-reads + re-parses OUTCOME.md from disk every call, (2) rewrites the status line, (3) calls `state::mutate` (which bumps revision again), (4) writes the rewritten file. The second ratify therefore genuinely re-parses; if the first ratify corrupted the file, the second would fail at `split_frontmatter`. Test also asserts (post-second-ratify) standalone fence line + `outcome check` succeeds + `ratifiable:true`. |
| `tests/foundation.rs::state_corrupt_envelope_on_invalid_state_json` | Asserts `STATE_CORRUPT` code + message structure? | **PASS.** Asserts `json["ok"] == false`, `json["code"] == "STATE_CORRUPT"`, `json["retryable"] == false`, and `message.contains("Failed to parse STATE.json")`. Triggers `src/state/mod.rs:84` (the `serde_json::from_str` branch inside `load`) by writing `"{ this is not json"` to STATE.json and running `codex1 status`. Exercises a distinct branch from round-1's `init_refuses_to_overwrite` (which hits `:159`, the init-write branch). |
| `tests/e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal` | Round-3 extended to assert CLOSEOUT.md content. Meaningful? | **PASS.** Asserts `CLOSEOUT.md` exists, contains the `"CLOSEOUT"` header, contains the mission id (`"demo"`), contains `T1` and `T4` (pre-replan + post-replan completed tasks), and contains the exact `terminal_at` timestamp read from STATE.json. A regression that skipped `atomic_write(CLOSEOUT.md)` while still bumping STATE would now fail this test. |

## P0

None.

## P1

None.

## P2

### P2-1: `CliError::ReviewFindingsBlock` envelope has no triggering integration test

- **Citation.** `src/cli/review/record.rs:57` constructs `CliError::ReviewFindingsBlock` when the `--findings-file` path does not exist: `if !p.is_file() { return Err(CliError::ReviewFindingsBlock { … }) }`. `src/core/error.rs:46,90` maps the variant to the canonical code string `"REVIEW_FINDINGS_BLOCK"`, listed in the frozen 18-code set (`iter4-cli-contract-audit.md` CC-3). The variant is NOT one of the three reserved variants covered by round-1 P1-1/P1-2 unit tests (`ConfigMissing`, `NotImplemented`, `ReplanRequired`) — those had no production construction site; `ReviewFindingsBlock` does have one.
- **Evidence.** `Grep "REVIEW_FINDINGS_BLOCK" crates/codex1/tests` yields exactly one hit: `tests/close.rs:304` inside `check_review_dirty_reports_review_findings_block`. That test asserts `b["code"] == "REVIEW_FINDINGS_BLOCK"` where `b` is an element of `data.blockers[]` in `close check`'s success envelope. The matching production site is `src/cli/close/check.rs:127`, which constructs a `Blocker { code: "REVIEW_FINDINGS_BLOCK", … }` as a hardcoded string literal for any review whose verdict is `Dirty`. This is structurally independent of `CliError::ReviewFindingsBlock`: different file, different code path (read-only close-check projection vs error raised from `review record`), different type (`Blocker` struct vs `CliError` variant). The string match is coincidence, not coverage.
- **Scrutiny of adjacent tests.** `tests/review.rs` has five `--findings-file <path>` invocations (lines 331, 372, 410, 446, 676); all pass an existing findings file. `tests/close.rs` has four (`382, 429, 451, 559`); all pass existing files. `tests/e2e_replan_trigger.rs:190` likewise. The clap-level negative tests `t7_clap_rejects_clean_and_findings_conflict` and `t8_clap_rejects_neither_clean_nor_findings` catch argument-shape errors but do not reach the file-existence check on line 56. No test in the suite invokes `review record --findings-file <nonexistent-path>` without `--clean`.
- **Brief-item mapping.** Priority 1 ("Every `CliError` variant constructed in production has a triggering test asserting envelope `code`.") — `CliError::ReviewFindingsBlock` is constructed in production and has no triggering test asserting its envelope `code`. Classify as P2 (partial coverage) rather than P1 (documented invariant without direct test) because the code string itself IS asserted elsewhere in the suite; only the specific envelope construction path at `review/record.rs:57` is untested. Round-1 P1-1/P1-2 were P1 precisely because the reserved variants had no test asserting the code string at all.
- **Distinction from round-3 P3-1 (REJECTed).** Round-3 P3-1 was defense-in-depth on `review_start_dry_run_enforces_expect_revision` — that test already triggers `CliError::RevisionConflict` and asserts `code`+`retryable`; the gap was optional `context.expected`/`context.actual` fields already covered by eight sibling tests. This P2-1 is different: the entire `CliError::ReviewFindingsBlock` construction path has zero integration tests, not just partial envelope-shape coverage. Not duplicative.
- **Suggested fix.** Add to `tests/review.rs` (helpers already in-file):
  ```rust
  #[test]
  fn record_with_missing_findings_file_returns_review_findings_block() {
      let s = Seeded::new();
      run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
      let err = run_err(
          s.path(),
          &[
              "review",
              "record",
              "T5",
              "--findings-file",
              "does/not/exist.md",
              "--mission",
              MISSION,
          ],
      );
      assert_eq!(err["ok"], false);
      assert_eq!(err["code"], "REVIEW_FINDINGS_BLOCK");
      assert_eq!(err["retryable"], false);
      assert!(
          err["message"]
              .as_str()
              .is_some_and(|m| m.contains("findings file not found")),
          "message should name the missing file: {err}"
      );
  }
  ```
  Assertions: `ok:false`, `code:REVIEW_FINDINGS_BLOCK`, `retryable:false` (variant does not set `retryable`), and message substring pinning the construction site's `"findings file not found: {path}"` format string at `src/cli/review/record.rs:58`. No new test file; one additional function in the existing `tests/review.rs`.

## P3

None. Round-3 P3-1 (`review_start_dry_run_enforces_expect_revision` context-field omission) and P3-2 (TOCTOU-concurrent tests for the other three `require_plan_locked`-in-closure paths) were explicitly REJECTed in round-3 and not re-surfaced here. No new style-only findings.

---

## Totals

Counts per unique finding (no cross-reviewer dedupe needed at the test-adequacy layer).

| Category | Count |
|----------|-------|
| P0       | 0     |
| P1       | 0     |
| P2       | 1     |
| P3       | 0     |
