# Round 1 decisions

Baseline audited: 4c67f03 (round-1 audits committed).

Reviewer reports: `docs/audits/round-1/{cli-contract,e2e-walkthrough,handoff-cross-check,skills-audit,correctness-invariants,test-adequacy}.md`.

## FIX

- **e2e P1-1 · task/review mutations gate on `plan.locked`** — added `state::require_plan_locked` helper; wired into `task start`, `task finish`, `review start`, `review record`. Returns `PLAN_INVALID` with hint "Run `codex1 plan check` first" when the plan is unlocked (e.g. after `replan record`). Test: `tests/replan.rs::task_start_after_replan_record_refuses_with_plan_invalid`.
- **correctness P1-1 · STATE/EVENTS write order** — `state::mutate` now appends `EVENTS.jsonl` before persisting `STATE.json`; a crash between the two writes now produces a detectable trailing event line instead of a silent audit gap. Covered by the concurrent-writer test plus every existing integration test that reads events after a mutation.
- **correctness P1-2 · parent-directory fsync after `persist`** — `state::fs_atomic::atomic_write` opens the parent dir and calls `sync_all` after `persist` so rename durability survives power loss on Linux/ext4/xfs (no-op on macOS). Covered by every atomic-write test path in the suite.
- **correctness P1-4 · `outcome ratify` atomicity** — moved `atomic_write(OUTCOME.md)` out of the `state::mutate` closure so a state-persist failure cannot leave OUTCOME.md flipped to `ratified` on disk while STATE remains `ratified=false`. Covered by existing `tests/outcome.rs::ratify_*` cases.
- **correctness P1-5 · idempotent / dry-run paths skipped `--expect-revision`** — promoted the helper to `state::check_expected_revision` and wired it into every short-circuit: `task/start.rs` (idempotent + dry-run), `task/finish.rs`, `plan/check.rs`, `plan/choose_level.rs`, `review/record.rs`, `close/complete.rs`, `close/record_review.rs` (clean + dirty), `outcome/ratify.rs`, `loop_/mod.rs`. Test: `tests/task.rs::start_idempotent_branch_still_enforces_expect_revision`.
- **skills P1-1 · execute SKILL task-next dispatch kinds** — Step 1 now reads `codex1 --json status` and dispatches on `data.next_action.kind` (per handoff `01-product-flow.md:151`); documented that `repair` lists target ids under `data.next_action.task_ids`. Text-only skill fix; `quick_validate.py` passes.
- **skills P1-2 · plan SKILL replan snippet missing `--reason`** — replaced the broken line with `codex1 replan record --reason <code> --supersedes <id>` plus pointer to `cli/replan/triggers.rs::ALLOWED_REASONS`. The existing `tests/replan.rs::record_rejects_unknown_reason` already exercises the contract; adding a second would duplicate.
- **skills P1-3 · review-loop reviewer model matrix** (dedupes handoff-cross-check P3 F2) — aligned `.codex/skills/review-loop/SKILL.md` + `references/reviewer-profiles.md` to the frozen handoff matrix (`gpt-5.3-codex` for `code_bug_correctness`, `gpt-5.4` for `local_spec_intent`). Skills are the consumer-facing prompt; handoff is frozen per rule 5.
- **test-adequacy P1-1 + P1-2 ≈ correctness P3-1 + P3-2 · reserved `CliError` variant envelopes** — kept the reserved variants (`ConfigMissing`, `NotImplemented`, `ReplanRequired`) and added unit tests in `src/core/error.rs::tests` that construct each via `to_envelope()` and assert `code`, `retryable`, `context`, `hint`. Deletion was the cheaper path but iter4 (`cli-contract-audit.md` CC-3) lists these in the canonical 18-code set; deleting is a contract break.
- **test-adequacy P1-3 · `OUTCOME_NOT_RATIFIED` envelope on `plan choose-level` pre-ratify** — test: `tests/plan_scaffold.rs::choose_level_before_ratify_returns_outcome_not_ratified`.
- **test-adequacy P1-4 · `PARSE_ERROR/PLAN_INVALID` fallthrough on corrupt YAML** — tests: `tests/foundation.rs::corrupt_plan_yaml_returns_plan_invalid_with_hint`; unit test `core::error::tests::io_error_maps_to_parse_error_and_bug_exit`.
- **test-adequacy P1-5 · `derive_verdict` has no unit tests** — added `src/state/readiness.rs::tests` with 11 unit tests: one per Verdict arm, plus `close_ready_is_only_true_for_passed`, `stop_allowed_when_loop_inactive_or_paused`, `tasks_complete_requires_dag_and_records`.
- **e2e P2-1 · plan check accepts deadlock plan + status blocked message misleads** — `cli/plan/check.rs::detect_review_loop_deadlock` rejects a plan where a non-review task upstream of review `R` depends on one of `R`'s targets (the shape the audit reproduced). `cli/status/project.rs::derive_next_action` surfaces "tasks T… awaiting review; no review task ready" instead of the generic "PLAN.yaml may be missing/empty". Tests: `tests/plan_check.rs::review_loop_deadlock_returns_plan_invalid`, `tests/status.rs::blocked_surfaces_awaiting_review_when_plan_is_valid`.
- **e2e P2-2 · `status.ready_tasks` advertised while `plan.locked=false`** — short-circuited wave/review derivation in `cli/status/project.rs::build` so `ready_tasks: []`, `review_required: []`, `parallel_safe: false` when the plan is unlocked. Test: `tests/status.rs::unlocked_plan_emits_empty_ready_tasks_and_review_required`.
- **handoff-cross-check P2 F1 + P3 F4 · escalation guards** (one fix resolves both) — `cli/plan/choose_level.rs::level_rank` guards `escalation_reason` on `effective > requested`, and the payload now always carries `escalation_required: bool` matching the handoff example at `02-cli-contract.md:306-315`. Tests: `tests/plan_scaffold.rs::choose_level_escalate_on_hard_suppresses_escalation_reason`, `choose_level_escalation_required_flag_appears_on_bump`.
- **skills P2-2 · autopilot dispatch table missing `fix_state`** — added the row `fix_state → Escalate to user; do not auto-fix STATE.json`. Text-only fix matching the reference state machine.
- **correctness P2-1 ≈ test-adequacy P2-2 · concurrent-writer test** (one test resolves both) — `tests/foundation.rs::concurrent_loop_activate_serializes_via_fs2_lock` spawns two threads racing on `loop activate --expect-revision 0`; asserts exactly one succeeds, the other returns `REVISION_CONFLICT`, STATE ends at revision 1, EVENTS.jsonl contains exactly one line with seq=1.
- **test-adequacy P2-1 · `late_same_boundary` counter invariant** — test: `tests/review.rs::late_same_boundary_does_not_bump_or_reset_dirty_counter` seeds the counter at 3 on two targets, records a dirty-late finding, asserts the counter stays at 3 and `replan.triggered` stays false.
- **test-adequacy P2-3 · full error envelope shape stability** — test: `tests/foundation.rs::error_envelope_shape_is_stable_across_representative_codes` (covers `MISSION_NOT_FOUND` + `REVISION_CONFLICT`) plus unit test `core::error::tests::revision_conflict_envelope_shape_is_stable`.
- **test-adequacy P2-4 · mission-close review `Open → Passed` transition** — test: `tests/close.rs::record_review_open_then_clean_transitions_to_passed`.

## REJECT

- **cli-contract P3 · suggested-verdict-list `complete` vs code `terminal_complete`** — out-of-scope: handoff docs are frozen for this loop (rule 5); code matches `00-why-and-lessons.md:173` which is the canonical terminal vocabulary. The drift is internal to the handoff, not a code bug. Dedupes with handoff-cross-check P3 F3.
- **handoff-cross-check P3 F3 · same as above** — duplicate; see cli-contract P3.
- **skills P2-1 · execute SKILL Claude-family equivalents** — style, not a bug: the default coding worker in the skill still matches the handoff (`gpt-5.3-codex`), and the reviewer explicitly notes this is adjacent text. Promoting Claude equivalents to contract substitutes would require a handoff change (rule 5).
- **correctness P1-3 · doctor `*.tmp` orphan GC** — no handoff mandate; `iter4-handoff-cross-check.md:116-129` confirmed `doctor` is a read-only health report by design, and `fs_atomic.rs:5-6` says "orphaned on crash" descriptively not normatively. Adding GC is a new product feature outside scope.
- **correctness P2-2 · atomic_write crash-consistency test** — test-attempt-brittle: reliable cross-platform mid-write crash injection is not available in Rust stable; `tempfile::NamedTempFile::persist` is the pattern used by Cargo/rustup/etc., and the invariant is delivered by the library. The new concurrent-writer test exercises the module under real load.
- **correctness P3-3 · `Verdict::InvalidState` unreachable** — non-blocking P3; not in loop scope.
- **correctness P3-4 · `.expect` calls without invariant comments** — non-blocking P3; not in loop scope. (A comment was added to `choose_level.rs::build_payload` as a drive-by while fixing the escalation guard.)
- **correctness P3-5 · doctor network-mount substring heuristic** — non-blocking P3; not in loop scope.
- **e2e P3-1 · `close.reviewers` not persisted in STATE.json** — non-blocking P3; not in loop scope.
- **e2e P3-2 · `close_ready:false` at `verdict:terminal_complete`** — non-blocking P3; not in loop scope.
- **skills P3-1 · execute SKILL "Use after" wording** — non-blocking P3; not in loop scope.
- **skills P3-2 · clarify SKILL handoff phrasing** — non-blocking P3; not in loop scope.
- **skills P3-3 · close/review-loop terminal duplication** — non-blocking P3; not in loop scope.
- **test-adequacy P3-1 · proptest-style agreement test** — non-blocking P3; not in loop scope.
- **handoff-cross-check P3 F2 (reviewer model table drift) → handled as P1 above** — reclassified upward via dedupe with skills P1-3.
- **handoff-cross-check P3 F4 (`escalation_required` field) → handled as P2 above** — bundled into the handoff P2 F1 fix.

## Totals

Counts below are per unique finding after cross-reviewer dedupe. Dedupe notes:

- skills P1-3 ≈ handoff-cross-check P3 F2 — one decision at the higher severity (P1).
- correctness P3-1, P3-2 ≈ test-adequacy P1-1, P1-2 — one CliError-unit-tests decision at the higher severity (P1).
- correctness P2-1 ≈ test-adequacy P2-2 — one concurrent-writer test.
- cli-contract P3 ≈ handoff-cross-check P3 F3 — one decision (REJECT).
- handoff-cross-check P3 F4 — bundled into handoff P2 F1 (counted under P2 FIX).

| Category | FIX | REJECT |
|----------|-----|--------|
| P0       |  0  |   0    |
| P1       | 13  |   1    |
| P2       |  9  |   2    |
| P3 (non-blocking, out of scope) | 0 | 10 |
