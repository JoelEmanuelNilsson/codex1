# Round 5 — test-adequacy audit

Reviewer: test-adequacy (6/6). Lens: `crates/codex1/tests/*.rs` + `src/**` unit tests. HEAD: `b08d461`. Prior decisions consumed: round-1, round-2, round-3, round-4. Rejected findings not re-surfaced (see dedicated notes below on `Verdict::InvalidState` and site-level variant duplication).

## Summary

- **Test count verified**: 184 integration `#[test]` (16 files) + 27 unit `#[test]` (5 files) = **211** (matches prompt: 209 + 2 round-4 additions).
- **Round-4 spot checks: 2-of-2 pass, non-vacuous.** Detail table below.
- **Zero new findings at P0 / P1 / P2.** Every `CliError` variant constructed in production has at least one triggering test asserting envelope `code`; every reachable `Verdict` arm is unit-tested; task and review lifecycle transitions, dirty-counter edge cases, concurrency invariants, error-envelope shape stability, and the Stop-hook contract are all covered at the bar established in round-1 and affirmed in rounds 2-4.
- **Both round-4 spot-check tests fire against the specific bug they guard** (verified below by inspecting the asserted substrings against the pre-fix parser and construction-site format string).
- **One candidate finding investigated and classified P3 / precedent-rejected** (not surfaced as a new finding): `CliError::ProofMissing` at `src/cli/close/record_review.rs:147` and `CliError::ParseError` at `src/cli/close/record_review.rs:44` (reachable because `close record-review` has clap `conflicts_with` but no `required_unless_present`). Both variants are already pinned at the envelope level via other triggering sites (`tests/task.rs:422` and `tests/plan_scaffold.rs:208,229,243,386`). Round-4 P2-1 explicitly framed the bar as "variant with zero CliError envelope triggering test," not per-construction-site. Round-1 P3-3/P3-4 and round-2/3 P3 rejections of analogous site-level gaps settle this at P3 / non-loop-scope. See Round-5 non-findings log at the bottom.

### Round-4 spot-check scrutiny

| Test | Scrutiny ask | Verdict |
|---|---|---|
| `tests/review.rs::review_packet_mission_summary_strips_yaml_block_scalar` (review.rs:716-762) | Seeds a block-scalar OUTCOME.md, asserts `|` absent, asserts content present. | **PASS — non-vacuous.** Seeds OUTCOME.md frontmatter with `interpreted_destination: |\n  Body line 1\n  Body line 2` (the exact shape round-4 identified as defeating the old substring parser). Asserts (a) `!summary.contains('|')`, (b) `!summary.starts_with('|')`, (c) `summary.contains("Body line 1")`, (d) `summary.contains("Body line 2")`, (e) `summary.contains("Body line 1\nBody line 2")`. Assertion (a) is the direct guard against the pre-fix bug (which emitted `|` into the output because `trim_end()` left the space between `:` and `|` intact at `review/packet.rs` pre-fix); (e) pins the post-fix `serde_yaml` semantics of a literal-block scalar (newline join, trailing `\n` trimmed by the new `trim()` at packet.rs:152). A regression that reverted to substring scanning would fail (a); a regression that accidentally swallowed the body (e.g. matched the wrong YAML key) would fail (c)-(e). |
| `tests/review.rs::review_record_findings_then_retry_returns_review_findings_block_envelope` (review.rs:773-809) | Asserts `code=REVIEW_FINDINGS_BLOCK`, `retryable=false`, `ok=false`, message content. | **PASS — non-vacuous.** All four prompt-required assertions present and tight: `err["ok"] == false`, `err["code"] == "REVIEW_FINDINGS_BLOCK"`, `err["retryable"] == false`, `message.contains("findings file not found")` AND `message.contains("does/not/exist.md")`. The message-substring asserts pin the format string at `src/cli/review/record.rs:58` (`format!("findings file not found: {}", p.display())`) from both sides: the literal phrase AND the caller-supplied path. A drift that merely dropped the path (logging only `"findings file not found"`) would fail the second substring assertion; a drift that silently changed the variant to some other error code would fail the `code` assertion. The test is the first envelope-level trigger for `CliError::ReviewFindingsBlock` (round-4 P2-1's closure) — prior `"REVIEW_FINDINGS_BLOCK"` string matches in the suite were all from `Blocker` struct instances, structurally independent of this CliError variant. |

## P0

None.

## P1

None.

## P2

None.

## P3

None. All round-3 P3 rejections and the round-4 skills-audit P3 rejection stand; no new style-level gaps worth re-surfacing. The per-construction-site duplication class (`PROOF_MISSING` at `close/record_review.rs:147`, `PARSE_ERROR` at `close/record_review.rs:44`) is documented below as prior-precedent REJECT rather than raised as a finding.

---

## Verification detail by prompt priority

Each priority was re-checked against the current tree at HEAD `b08d461`.

### Priority 1 — every `CliError` variant constructed in production has an envelope-code test

Mapped each variant (enumerated at `src/core/error.rs:24-76`) to at least one triggering test asserting `code`:

| Variant | Envelope-code test (representative) |
|---|---|
| `OutcomeIncomplete` | `tests/outcome.rs:125,212,254` |
| `OutcomeNotRatified` | `tests/plan_scaffold.rs:502` (integration) + round-1 unit coverage via `derive_verdict` arms |
| `PlanInvalid` | `tests/plan_check.rs:221,276,325,473,521,573,780`, `tests/replan.rs:199,229,263,456`, `tests/loop_.rs:342,359`, `tests/foundation.rs:377` |
| `DagCycle` | `tests/plan_check.rs:377` |
| `DagMissingDep` | `tests/plan_check.rs:426` |
| `TaskNotReady` | `tests/task.rs:314`, `tests/loop_.rs:279,320,404`, `tests/review.rs:259` |
| `ProofMissing` | `tests/task.rs:422` (variant-level coverage) |
| `ReviewFindingsBlock` | `tests/review.rs:792` (round-4 addition) |
| `ReplanRequired` | `src/core/error.rs::tests::replan_required_envelope_shape_is_stable` (reserved; no production construction site in the CLI flow — `replan record` mutates state but does not raise this variant) |
| `CloseNotReady` | `tests/close.rs:460,481,691` |
| `StateCorrupt` | `tests/foundation.rs:95,257` |
| `RevisionConflict` | `tests/task.rs:592,620,656`, `tests/loop_.rs:429,454`, `tests/close.rs:536,762`, `tests/foundation.rs:221,332`, `tests/plan_check.rs:611`, `tests/outcome.rs:371`, `tests/replan.rs:339`, `tests/review.rs:558,579` |
| `StaleReviewRecord` | `tests/review.rs:487` |
| `TerminalAlreadyComplete` | `tests/e2e_full_mission.rs:490`, `tests/close.rs:669`, `tests/review.rs:507` |
| `ConfigMissing` | `src/core/error.rs::tests::config_missing_envelope_shape_is_stable` (reserved; round-1 P1-1/P1-2) |
| `MissionNotFound` | `tests/foundation.rs:146,199` |
| `ParseError` | `tests/plan_scaffold.rs:208,229,243,386` (integration) + `src/core/error.rs::tests::io_error_maps_to_parse_error_and_bug_exit` (unit) |
| `NotImplemented` | `src/core/error.rs::tests::not_implemented_envelope_shape_is_stable` (reserved) + `tests/status_close_agreement.rs:439` (integration) |
| `Io / Json / Yaml` (transparent) | `src/core/error.rs::tests::io_error_maps_to_parse_error_and_bug_exit` pins the `→ PARSE_ERROR` mapping and `ExitKind::Bug` classification |

Result: every variant has an envelope-code assertion.

### Priority 2 — every `Verdict` branch has a unit test

`src/state/readiness.rs::tests` covers seven of eight arms (unit tests at lines 149-232):

- `TerminalComplete`: `derive_verdict_terminal_complete_wins_over_everything`
- `NeedsUser` (unratified outcome): `derive_verdict_unratified_outcome_is_needs_user`
- `NeedsUser` (unlocked plan): `derive_verdict_unlocked_plan_is_needs_user`
- `Blocked` (replan triggered): `derive_verdict_replan_triggered_is_blocked`
- `Blocked` (dirty review): `derive_verdict_dirty_review_is_blocked`
- `ContinueRequired`: `derive_verdict_tasks_incomplete_is_continue_required`
- `ReadyForMissionCloseReview`: `derive_verdict_tasks_complete_not_started_is_ready_for_mission_close_review`
- `MissionCloseReviewOpen`: `derive_verdict_tasks_complete_review_open_is_mission_close_review_open`
- `MissionCloseReviewPassed`: `derive_verdict_tasks_complete_review_passed_is_mission_close_review_passed`

The eighth arm, `Verdict::InvalidState`, is referenced in `src/cli/status/project.rs:125` as a defensive next-action branch but is never produced by `derive_verdict` at `src/state/readiness.rs:40-66`. Round-1 correctness P3-3 flagged this and was REJECTed as out-of-scope unreachable; decision stands. Not re-surfaced.

### Priority 3 — task + review lifecycle transitions

Task lifecycle covered (`tests/task.rs`): ready→in_progress (line 285), deps-incomplete rejection (307), idempotent start (317), finish→complete without review target (336), finish→awaiting_review with review target (366), missing-proof error (404), dry-run (534, 548), expect-revision mismatch (575, 598, 627).

Review lifecycle covered (`tests/review.rs`): pending→records→clean→counter reset (281, 395), pending→findings→dirty counter bump + file copy (317), 6-dirty→replan trigger (356), clap conflict rejection (436, 461), stale-superseded (477), terminal-already-complete (498), dry-run (510), expect-revision mismatch (542, 562), post-record status (583), packet content (609), late-same-boundary flag + no-bump semantics (629, 649), start-before-targets-finished (254), null-record status (700), block-scalar mission-summary (715), findings-file envelope (772).

Mission-close lifecycle covered (`tests/close.rs`): check arms (208, 230, 260, 282, 346), record-review clean→passed (314), findings→counter (370, 416), not-ready refusal (463), dry-run (484), expect-revision (515), Open→Passed re-entry (546 — round-1 test-adequacy P2-4 addition), complete success/twice/not-ready/dry-run/expect-revision (591, 648, 673, 699, 735), status/close-check agreement (771 — enforces the Foundation shared-predicate invariant).

### Priority 4 — concurrency invariants covered

- `tests/foundation.rs::concurrent_loop_activate_serializes_via_fs2_lock` (274): two threads race `loop activate --expect-revision 0`; exactly one succeeds, the other returns `REVISION_CONFLICT`, STATE bumps to revision 1, EVENTS.jsonl has exactly one line with seq=1. (Round-1 correctness P2-1 / test-adequacy P2-2 dedupe.)
- `tests/foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant` (393): races `task start T2` against `replan record --supersedes T2 --reason six_dirty` across 4 iterations, asserts final shape never `!plan.locked && tasks.T2.status == "in_progress"`. (Round-2 correctness P1-1 addition.)
- The round-3 P3-2 rejection (only one of four TOCTOU sites has a concurrent test) settled the bar at one concurrent test per mutation-module invariant; precedent stands.

### Priority 5 — dirty counter edge cases

Covered at both the per-target layer and the mission-close layer:

- Bump: `tests/review.rs:317` (`t4_record_findings_increments_dirty_and_copies_file`), `tests/close.rs:371`.
- Reset on clean: `tests/review.rs:281,313` (clean resets to 0).
- Reset on replan: `tests/replan.rs:159` (`consecutive_dirty_by_target` cleared).
- 6-threshold trigger (per-target): `tests/review.rs:356` (`t5_six_dirty_triggers_replan`), `tests/e2e_replan_trigger.rs:208`.
- 6-threshold trigger (mission-close): `tests/close.rs:417` (`record_review_six_consecutive_dirty_triggers_replan`).
- Interruption by clean: `tests/review.rs:395` (`t6_clean_interrupts_dirty_streak`).
- Late-same-boundary no-bump + no-reset: `tests/review.rs:649` (round-1 test-adequacy P2-1 addition — covers the subtle invariant that a dirty-late record on already-recorded targets must neither bump nor reset the counter).

### Priority 6 — error envelope shape stability

`tests/foundation.rs::error_envelope_shape_is_stable_across_representative_codes` (188) pins the full JsonErr shape (`ok`, `code`, `message`, `retryable`, `hint`, `context`) for `MISSION_NOT_FOUND` (hint: yes, retryable: false, context: null) and `REVISION_CONFLICT` (retryable: true, context has `expected`+`actual`, hint: yes). `src/core/error.rs::tests::revision_conflict_envelope_shape_is_stable` (unit) and the three reserved-variant unit tests provide second-layer coverage. Round-2 REJECTed the "table-driven every-variant envelope round-trip" as P3 out-of-loop-scope; precedent stands.

### Priority 7 — Stop-hook contract

`tests/ralph_hook.rs` covers all five decision branches of `scripts/ralph-stop-hook.sh`:

- `stop.allow:true` → exit 0 (line 103)
- `stop.allow:false` → exit 2, stderr explains (line 120)
- Empty status output → exit 0, warning (line 141)
- `codex1` missing from PATH → exit 0, warning (line 159)
- Malformed JSON without `stop.allow` → exit 0 default (line 175)
- Script file executable bit set (line 194)

---

## Round-5 non-findings log (classified P3 / precedent-rejected)

Recorded here for audit completeness so the next reviewer does not re-open the same threads; **not** counted in the totals table.

1. **`PROOF_MISSING` at `src/cli/close/record_review.rs:147`** — reached by `close record-review --findings-file <nonexistent-path>`. The variant is already covered at `tests/task.rs:422` via `task finish --proof-file <nonexistent>`. The Round-1 P1-1/P1-2 bar and Round-4 P2-1 bar are variant-level, not site-level. Adding a second site-level test is cosmetic. Precedent (round-1 correctness P3-4, round-2/3/4 P3 rejections) settles this.

2. **`PARSE_ERROR` at `src/cli/close/record_review.rs:44` (`(false, None)` branch)** — `close record-review` has clap `conflicts_with` but no `required_unless_present`, so running `close record-review --mission demo` with no flags passes clap and reaches the internal `ParseError`. Variant is already covered at `tests/plan_scaffold.rs:208,229,243,386`. Same site-level-vs-variant-level precedent applies. Not raised.

3. **Asymmetric clap enforcement between `review record` and `close record-review`** — `review record` has both `conflicts_with` and `required_unless_present` (see `src/cli/review/mod.rs:40-48`), so its internal `CliError::ParseError` at `src/cli/review/record.rs:42` is unreachable via the CLI; `close record-review` has only `conflicts_with` (see `src/cli/close/mod.rs:52`), so its internal `ParseError` branches at `record_review.rs:41,44` ARE reachable. This is a CLI-contract shape drift, not a test-adequacy finding — it is outside this reviewer's lens. If surfaced, it is a cli-contract-layer P3 at best (no behavioral bug; both paths error out correctly — just via different error-producing layers), and the round-1 cli-contract P3 rejection of terminal-vocabulary drift precedents handle similar cases.

4. **`Verdict::InvalidState`** — round-1 REJECT P3-3 stands.

---

## Totals

Counts per unique finding (no cross-reviewer dedupe needed at the test-adequacy layer).

| Category | Count |
|----------|-------|
| P0       | 0     |
| P1       | 0     |
| P2       | 0     |
| P3       | 0     |
