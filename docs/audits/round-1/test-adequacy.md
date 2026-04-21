# Round 1 — test-adequacy audit

## Summary

Scope: assessed `/Users/joel/codex1/crates/codex1/tests/*.rs` (16 files, 170 integration tests) plus unit tests in `src/**` (10 lib tests) against the invariants mandated by `docs/codex1-rebuild-handoff/02-cli-contract.md` and `03-planning-artifacts.md`. Build passes; the suite compiles and enumerates 180 total tests.

Coverage is broad across the CLI surface and reasonably deep on the happy paths and well-documented error cases (REVISION_CONFLICT, PLAN_INVALID, DAG_CYCLE, DAG_MISSING_DEP, TASK_NOT_READY, PROOF_MISSING, STALE_REVIEW_RECORD, TERMINAL_ALREADY_COMPLETE, CLOSE_NOT_READY, OUTCOME_INCOMPLETE, MISSION_NOT_FOUND, STATE_CORRUPT). Status/close-check agreement is over-provisioned (24 cases in `close.rs` + 21 in `status_close_agreement.rs`) — it is enumerated-matrix, not generator-based, but clearly not a single-case test, so it satisfies the handoff's "must share readiness logic" bar.

Real gaps cluster in three places: (1) six `CliError` variants have no test that provokes them and asserts the JSON envelope `code` — notably `ConfigMissing`, `NotImplemented` (on a live path), `OutcomeNotRatified`, and the `Io/Json/Yaml` → `PARSE_ERROR` fallthroughs. (2) `src/state/readiness.rs::derive_verdict` has **zero** unit tests; every branch is only hit via CLI integration, which is fragile if the CLI projection layer is refactored. (3) The `late_same_boundary` classification is asserted but the counter-invariant it guards (item 7c of the brief — late records must NOT bump the dirty counter) is not asserted.

No P0: no fundamental correctness property is entirely untested. The fs2 lock is exercised by every mutating test, the atomic-write temp-file flow runs on every mutation, and the verdict derivation is reached thousands of times across the suite.

**Totals: 0 P0, 5 P1, 4 P2, 1 P3.**

## P0

None.

## P1

### P1-1: `CliError::ConfigMissing` has no triggering test asserting envelope `code`

- **Citation:** `docs/codex1-rebuild-handoff/02-cli-contract.md` §Error Shape mandates a stable error set; `src/core/error.rs:59-60,97,133-136` defines `ConfigMissing` with code `CONFIG_MISSING` and a hint.
- **Evidence:** `Grep "CONFIG_MISSING" crates/codex1/tests/**` returns 0 hits. No test forces the code path that would construct this error (it exists as a reserved variant but the suite never asserts the envelope shape for it).
- **Suggested fix:** Add a unit test `cli::doctor::tests::reports_config_missing_when_forced` or an integration `foundation.rs::doctor_reports_config_missing_with_envelope` that invokes whatever path constructs `ConfigMissing` and asserts `json["ok"]==false`, `json["code"]=="CONFIG_MISSING"`, `json["hint"]` is a non-empty string, `json["retryable"]==false`. If the variant is genuinely unreachable today, the finding shifts to "remove the variant" — but the code still claims it's part of the contract, so the test should exist.

### P1-2: `CliError::NotImplemented` is never asserted on a live path

- **Citation:** `src/core/error.rs:68-69,100,137-140,154` defines `NotImplemented` with code `NOT_IMPLEMENTED` and context `{"command": ...}`.
- **Evidence:** The only hit for `NOT_IMPLEMENTED` in the test tree is `status_close_agreement.rs:437-441`, which **tolerates** it as a stub signal and skips the assertion when it appears — no test actively provokes it and asserts the envelope shape. As Phase B has landed every advertised command, no surface is expected to return `NotImplemented`, yet the contract still lists it.
- **Suggested fix:** Either remove the variant (the cleanest contract change) or add a narrow unit test `core::error::tests::not_implemented_envelope_shape` that constructs `CliError::NotImplemented { command: "x".into() }`, calls `.to_envelope()`, and asserts `code=="NOT_IMPLEMENTED"`, `context["command"]=="x"`, `retryable==false`, and `hint` is populated.

### P1-3: `CliError::OutcomeNotRatified` envelope never asserted

- **Citation:** `src/core/error.rs:31,84` defines the variant with code `OUTCOME_NOT_RATIFIED`. `src/cli/plan/choose_level.rs:25-28` returns it when `state::load(&paths).outcome.ratified` is false. Handoff §Plan Commands implies planning is gated on ratification.
- **Evidence:** `Grep "OUTCOME_NOT_RATIFIED" crates/codex1/tests/**` returns only `close.rs:223-224`, where it's asserted as an element of `data.blockers[*].code` inside `close check` output — a different surface (blocker list built by `close::check`), not the top-level error-envelope `code` produced by `choose-level` pre-ratify. No test runs `codex1 plan choose-level --mission demo --level medium` against a fresh (unratified) mission and asserts the envelope shape.
- **Suggested fix:** Add `plan_scaffold.rs::choose_level_before_ratify_returns_outcome_not_ratified` that `init_demo`'s but skips the ratify step, runs `codex1 plan choose-level --mission demo --level medium`, and asserts `json["ok"]==false`, `json["code"]=="OUTCOME_NOT_RATIFIED"`, `!output.status.success()`. The existing helper `seed_valid_outcome + outcome ratify` chain in that file makes this a one-line variant of `init_demo`.

### P1-4: `Io/Json/Yaml` transparent errors (`PARSE_ERROR` fallthrough) never triggered by corrupt state

- **Citation:** `src/core/error.rs:70-75,101,162` routes `Io`, `serde_json::Error`, and `serde_yaml::Error` to code `PARSE_ERROR` with `ExitKind::Bug`. `src/state/mod.rs:46-47,72-74` converts parse failures into `CliError::StateCorrupt` instead, so in practice `PARSE_ERROR` is reached via Io (e.g., a filesystem-level read failure) or via a direct `Yaml`/`Json` pass-through from plan/outcome parsing. Handoff `02-cli-contract.md` mandates predictable errors for every command.
- **Evidence:** Current `PARSE_ERROR` assertions (`plan_scaffold.rs:208,229,243,386`) all come from the bespoke level-parsing layer (explicitly constructing `CliError::ParseError`). No test seeds a truncated/invalid `PLAN.yaml` byte stream or a corrupt `OUTCOME.md` that would surface a raw `serde_yaml::Error` through the transparent conversion, and no test exercises the `Io` path (e.g., unreadable state file with wrong permissions). `foundation.rs::init_refuses_to_overwrite` hits `STATE_CORRUPT`, not `PARSE_ERROR`. So the `#[error(transparent)]` transitive conversion is untested.
- **Suggested fix:** Add `plan_check.rs::corrupt_yaml_returns_parse_error` that writes `"tasks:\n  - id: T1\n    depends_on: [unterminated"` (invalid YAML) as PLAN.yaml and asserts `json["code"]=="PARSE_ERROR"` and `!output.status.success()`. Plus a unit test `core::error::tests::io_error_maps_to_parse_error` that constructs `CliError::Io(std::io::Error::from(ErrorKind::PermissionDenied))` and verifies `.code() == "PARSE_ERROR"` and `.kind()` is `ExitKind::Bug`.

### P1-5: `state::readiness::derive_verdict` has no unit tests — every branch is only hit via CLI integration

- **Citation:** Brief item 2: "For each [of ~10 distinct outcomes], is there a unit test? Missing → P1." The source file `src/state/readiness.rs` documents ten verdict outcomes (`invalid_state, terminal_complete, needs_user×2, blocked×2 via replan/dirty, ready_for_mission_close_review, mission_close_review_open, mission_close_review_passed, continue_required`).
- **Evidence:** `Grep "#\[test\]|mod tests" crates/codex1/src/state/readiness.rs` returns zero matches. The branches are covered by integration fixtures (21 in `status_close_agreement.rs::fixtures()`, 24 in `close.rs::agreement_cases()`, plus every `status.rs` test), but there is no direct unit test that constructs a `MissionState` and asserts `derive_verdict(&state) == Verdict::Blocked` et al. without shelling out. A refactor of the status/close-check projection layer could silently break verdict derivation while every integration test still passes if the projection happens to re-route correctly.
- **Suggested fix:** Add a `src/state/readiness.rs` unit-test module `mod tests` with one `#[test]` per `Verdict` arm: `derive_verdict_terminal_complete_wins_over_everything`, `derive_verdict_unratified_outcome_is_needs_user`, `derive_verdict_unlocked_plan_is_needs_user`, `derive_verdict_replan_triggered_is_blocked`, `derive_verdict_dirty_review_is_blocked`, `derive_verdict_tasks_complete_and_review_not_started_is_ready_for_mission_close_review`, `derive_verdict_tasks_complete_and_review_open_is_mission_close_review_open`, `derive_verdict_tasks_complete_and_review_passed_is_mission_close_review_passed`, `derive_verdict_tasks_incomplete_is_continue_required`. Each constructs a minimal `MissionState` and asserts the expected `Verdict` variant directly. Also add `close_ready_is_only_true_for_passed`, `stop_allowed_matches_spec`, `tasks_complete_requires_dag_and_records`.

## P2

### P2-1: `late_same_boundary` records bumping/not-bumping the dirty counter (item 7c) is not asserted

- **Citation:** Brief item 7c: "a `late_same_boundary` record NOT affecting the counter". Implementation at `src/cli/review/record.rs:276-282` gates both `apply_clean` and `apply_dirty` on `matches!(category, ReviewRecordCategory::AcceptedCurrent)`, so a `LateSameBoundary`-classified Dirty record must leave `consecutive_dirty_by_target` untouched.
- **Evidence:** `review.rs::late_same_boundary_is_flagged` (lines 604–616) asserts the category but **not** the counter invariant. It only checks `ok["data"]["category"] == "late_same_boundary"` and `warnings` — it never reads `state.replan.consecutive_dirty_by_target` after the record to confirm it remained at zero. This is a vacuous test for the claimed invariant.
- **Suggested fix:** Extend the existing test (or add sibling `review.rs::late_same_boundary_does_not_bump_dirty_counter`): after the `review record T5 --findings-file …` call (seed findings instead of `--clean` so verdict would otherwise be dirty), read STATE.json and assert `state["replan"]["consecutive_dirty_by_target"].as_object().map_or(true, |m| m.values().all(|v| v == 0))` — i.e., no counter incremented. Also seed the counter with a prior non-zero value (e.g., `{"T2": 3}`) and assert it stays at 3 after the late record.

### P2-2: fs2 exclusive-lock serialization under concurrency is not directly exercised

- **Citation:** `state/mod.rs:1-16` docstring promises "exclusive `fs2` lock on `STATE.json.lock` … serializes them". Brief item 5 flags this as P2 if absent.
- **Evidence:** Every mutating test runs a single `codex1` subprocess and sees the lock in uncontended form. No test forks two processes racing on the same mission to verify the second one either (a) blocks until the first finishes, or (b) reads the updated revision and surfaces `REVISION_CONFLICT`. The lock is *acquired* on every test, but the serialization invariant is unproven.
- **Suggested fix:** Add `foundation.rs::concurrent_mutations_serialize_via_fs2_lock` that spawns two `codex1 loop activate --mission demo --expect-revision 0` children simultaneously via `std::process::Command::spawn`, collects both outputs, and asserts exactly one succeeds (revision=1) while the other returns `REVISION_CONFLICT` with `context.actual == 1`. This proves both (a) lock serialization and (b) `--expect-revision` fail-closed behavior under contention. Gracefully skip on Windows where fs2 semantics differ.

### P2-3: No test asserts the full error envelope shape `{ok, code, message, hint, retryable, context}`

- **Citation:** Brief item 9; handoff `02-cli-contract.md` §Error Shape pins the shape; `src/core/envelope.rs::JsonErr` is the serializer.
- **Evidence:** Across the suite, `hint` is asserted only twice (`loop_.rs:360`, `foundation.rs:147`), `retryable` asserted in a handful of REVISION_CONFLICT tests, `context` asserted for a few variants (`RevisionConflict`, `ProofMissing`, `DagCycle`, `DagMissingDep`, `NotImplemented`, duplicate_ids), and no single test asserts all four non-code fields present and correctly-typed per a given error path. A regression that drops `hint` serialization or flips `retryable` on an accidental `#[serde(skip)]` would not be caught.
- **Suggested fix:** Add `foundation.rs::error_envelope_shape_is_stable` that invokes three representative error paths (one with `hint`: `MISSION_NOT_FOUND`; one with `context`: `REVISION_CONFLICT`; one without either: `OUTCOME_NOT_RATIFIED`), and for each asserts (a) `ok == false`, (b) `code` is the expected string, (c) `message` is non-empty, (d) `retryable` is the expected bool, (e) `hint` type is as expected (string for MISSION_NOT_FOUND / null for no-hint variants), (f) `context` type matches (object with `expected`/`actual` for REVISION_CONFLICT). Also add a unit test `core::envelope::tests::json_err_serializes_all_fields` that builds `JsonErr::new(...)` with every field populated and round-trips through `serde_json::to_value` to assert no fields are silently dropped.

### P2-4: Task-lifecycle transitions are covered implicitly but not with one-test-per-edge coverage

- **Citation:** Brief item 4. `TaskStatus` = `Pending, Ready, InProgress, AwaitingReview, Complete, Superseded`. Review lifecycle = `MissionCloseReviewState` (`NotStarted, Open, Passed`) and `ReviewVerdict` / `ReviewRecordCategory`.
- **Evidence:** Direct coverage exists for: `Pending → InProgress` (`task.rs::start_ready_task_transitions_to_in_progress`), `InProgress → Complete` (`task.rs::finish_no_review_target_transitions_to_complete`), `InProgress → AwaitingReview` (`task.rs::finish_with_review_target_transitions_to_awaiting_review`), `AwaitingReview → Complete` (`review.rs::t3_record_clean_transitions_targets_and_resets_streak`), `AwaitingReview → Superseded` (`replan.rs::record_six_dirty_supersedes_tasks_and_unlocks_plan`), and review lifecycle `NotStarted → Open` via dirty (`close.rs::record_review_findings_writes_review_file_and_bumps_counter`), `NotStarted → Passed` via clean (`close.rs::record_review_clean_transitions_state_to_passed`). **Not directly tested:** (a) `Pending → Ready` — this is *derived*, not stored, so arguably untestable by design; but `task.rs::next_multi_ready_reports_wave` confirms `Ready` is projected when deps are Complete, so the derivation path is covered. (b) No test hits `Open → Passed` directly as a second main-thread record (the tests only go `NotStarted → Passed` or `NotStarted → Open`). (c) No test exercises a task restarting from `Complete` due to a dependency's mid-stream supersede.
- **Suggested fix:** Add `close.rs::record_review_open_then_clean_transitions_to_passed`: seed mission ready-for-review, record dirty findings (→ Open), then record `--clean` (→ Passed), and assert the intermediate state transitions correctly. Plus confirm the Superseded-return-to-AwaitingReview pattern used by `e2e_replan_trigger.rs::reset_target` is a legitimate transition by explicitly documenting/testing it in an integration test named `replan.rs::task_can_be_reset_to_awaiting_review_after_supersede_during_replan` — the current usage only tests a test-helper, not a CLI path.

## P3

### P3-1: Status/close-check agreement is enumerated-matrix, not `proptest`-generated

- **Citation:** Brief item 3: "If it's a single-case test, not a property test → P2". `close.rs::status_and_close_check_always_agree` enumerates 24 states; `status_close_agreement.rs::status_agrees_with_readiness_helpers_for_all_fixtures` enumerates 21.
- **Evidence:** 45 hand-curated fixtures is plainly not single-case, so the finding does not rise to P2. It's a style P3 because a generator-based property test (e.g., `proptest` over `arb_mission_state()`) could catch permutations the curated list missed — but the curated list already includes fresh, ratified-only, plan-locked, replan-triggered, dirty-review, pending/in-progress/ready/complete/superseded/awaiting_review task combinations, mission-close-review NotStarted/Open/Passed, terminal, and several crossovers. Coverage is high; generator-style is a nicety.
- **Suggested fix:** Optional — add `close.rs::status_and_close_check_agree_proptest` using `proptest = "1"` with an `arb_mission_state()` strategy that mutates one field at a time (ratified, plan_locked, replan.triggered, close.review_state, close.terminal_at, a BTreeMap of TaskStatus, a BTreeMap of ReviewVerdict). Assert agreement on every generated case. Not required for the round-1 gate.

---

### Notes on items deliberately not flagged

- **Item 6 (crash recovery via `doctor` orphan cleanup):** The handoff does not mandate `.tmp` orphan cleanup as a `doctor` behavior. `src/cli/doctor.rs` is a pure read-only health report (version, config, PATH, writable `~/.local/bin`, network-mount warning). No finding.
- **Item 8 (F11 upgrade-in-place trap):** `plan_check.rs::plan_check_backfills_missing_task_ids_and_then_stays_idempotent` (lines 660–710) exists and regresses the exact trap called out in `docs/audits/iter4-cli-contract-audit.md` F1. Fixture 21 in `status_close_agreement.rs` (`one_task_done_out_of_four`) regresses the sibling F8 semantic bug. Full coverage.
- **Item 10 (stop-hook):** `ralph_hook.rs` drives the script with five canned JSON payloads (allow=true → exit 0, allow=false → exit 2, empty output → exit 0, missing codex1 → exit 0 with warning, missing `stop.allow` field → exit 0). `e2e_ralph_contract.rs` duplicates the first five checks with slightly different PATH hygiene. Not stubbed, not smoke-tested. Covered.
- **Item 7a/b/d (dirty counter standard paths):** `review.rs::t5_six_dirty_triggers_replan` covers (a) six dirty → REPLAN_REQUIRED; `review.rs::t6_clean_interrupts_dirty_streak` covers (b) clean reset; `e2e_replan_trigger.rs::e2e_six_dirty_reviews_trigger_replan_and_record_clears_counters` covers (d) counter clear after replan record. Only (c) `late_same_boundary` is missing — flagged as P2-1 above.

---

Relevant source paths surveyed:
- `/Users/joel/codex1/crates/codex1/src/core/error.rs` — 18 CliError variants, code table at L82-102
- `/Users/joel/codex1/crates/codex1/src/core/envelope.rs` — JsonOk/JsonErr
- `/Users/joel/codex1/crates/codex1/src/state/readiness.rs` — 8 Verdict arms, no unit tests
- `/Users/joel/codex1/crates/codex1/src/state/mod.rs` — fs2 locking flow (L134-154)
- `/Users/joel/codex1/crates/codex1/src/cli/review/record.rs` — late_same_boundary gate at L276-282
- `/Users/joel/codex1/crates/codex1/src/cli/plan/choose_level.rs` — OutcomeNotRatified trigger at L27
- `/Users/joel/codex1/crates/codex1/tests/*.rs` — 16 integration files (170 tests)
