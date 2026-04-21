# Round 2 — test-adequacy audit

## Summary

Round 1 added ~19 tests (170 → 200). I verified the round-1 additions against the round-2 scrutiny checklist and they are all meaningful, not vacuous:

- **`src/state/readiness.rs::tests`** — 11 unit tests. One per reachable `Verdict` arm (`TerminalComplete`, `NeedsUser` for unratified outcome, `NeedsUser` for unlocked plan, `Blocked` for replan-triggered, `Blocked` for dirty review, `ContinueRequired`, `ReadyForMissionCloseReview`, `MissionCloseReviewOpen`, `MissionCloseReviewPassed`) plus `close_ready_is_only_true_for_passed`, `stop_allowed_when_loop_inactive_or_paused`, `tasks_complete_requires_dag_and_records`. Each test constructs a minimal `MissionState` via `MissionState::fresh` and asserts the exact arm. `InvalidState` is unreachable by construction (round-1 correctness P3-3 rejected).
- **`src/core/error.rs::tests`** — 5 unit tests. Reserved variants `ConfigMissing`, `NotImplemented`, `ReplanRequired` each build via `to_envelope()` and assert `code`, `retryable`, plus at least one of `hint`/`context`. `io_error_maps_to_parse_error_and_bug_exit` asserts both the code coercion and `ExitKind::Bug`. `revision_conflict_envelope_shape_is_stable` asserts `ok=false`, `code`, `retryable=true`, `context.expected`, `context.actual`, `hint`.
- **`tests/foundation.rs::concurrent_loop_activate_serializes_via_fs2_lock`** — spawns two threads racing on `loop activate --expect-revision 0`, asserts exactly one success, the failing one returns `REVISION_CONFLICT`, `STATE.json` revision ends at 1, and `EVENTS.jsonl` has exactly one line with `seq=1`. Not a luck-pass: the symmetric `--expect-revision 0` plus post-run invariants catch any serialization break.
- **`tests/foundation.rs::error_envelope_shape_is_stable_across_representative_codes`** — asserts `ok`, `code`, `message`, `retryable`, `hint` for `MISSION_NOT_FOUND`, and `ok`, `code`, `retryable=true`, `context.expected`, `context.actual`, `hint` for `REVISION_CONFLICT`. All 6 envelope fields are covered across the two paths (envelope serialization skips null `context`/`hint`, so per-path all-6 is not achievable and not required).
- **`tests/replan.rs::task_start_after_replan_record_refuses_with_plan_invalid`** — asserts exit-fail, `code == "PLAN_INVALID"`, and hint substring `"plan check"`. Also flips `plan.locked=true` and re-runs `task start` to prove the guard keys off exactly that flag.
- **`tests/status.rs::unlocked_plan_emits_empty_ready_tasks_and_review_required`** — seeds `T1::Ready`, `T2::Pending`, asserts `plan_locked=false`, `verdict="needs_user"`, `next_action.kind="plan"`, `ready_tasks == []`, `review_required == []`, `parallel_safe=false`.
- **`tests/plan_check.rs::review_loop_deadlock_returns_plan_invalid`** — constructs the reproducing shape (T4 review depends on [T1,T2,T3] with review_target [T1,T2,T3]; T3→T2→T1). Production emits `"review-loop deadlock: …"`; the test's `message.contains("deadlock") || message.contains("review-loop")` is satisfied by the actual production string. Also re-runs `plan check` to confirm idempotent rejection (no lock leak).
- **`tests/plan_scaffold.rs::choose_level_escalate_on_hard_suppresses_escalation_reason`** — asserts `requested_level="hard"`, `effective_level="hard"`, `escalation_reason` absent (via `.get().is_none()`), `escalation_required=false`. Paired test `choose_level_escalation_required_flag_appears_on_bump` asserts the field flips to `true` on a real escalation.
- **`tests/review.rs::late_same_boundary_does_not_bump_or_reset_dirty_counter`** — pre-seeds counter `{T2:3, T3:3}`, triggers `late_same_boundary` via `bump_revision_without_mutation`, asserts counter stays at 3 for both targets AND `replan.triggered == false`.
- **`tests/close.rs::record_review_open_then_clean_transitions_to_passed`** — drives `NotStarted → Open` (dirty) then `Open → Passed` (clean), asserts `review_state` at each step.

**Iter4 regression**: `tests/plan_check.rs::plan_check_backfills_missing_task_ids_and_then_stays_idempotent` still present and asserts backfill bump + subsequent idempotent behavior.

**Stop-hook contract**: `tests/ralph_hook.rs` + `tests/e2e_ralph_contract.rs` still cover `stop.allow=true → exit 0`, `stop.allow=false → exit 2 with "blocking" stderr`, and missing-field → conservative exit 0.

The suite now exercises the invariants the handoff mandates. No new P0/P1/P2 findings; one P3 offered as optional follow-up.

## P0

None.

## P1

None.

## P2

None.

## P3

### P3-1 · CliError round-trip coverage could be reinforced with an assert-once-per-variant table

**Context.** Round 1 closed the explicit gap by hand-adding unit tests for the three reserved variants plus two representative passthrough cases. The remaining variants are covered indirectly via integration tests (each hits `code` and usually `context`), but there is no single table-driven test that enumerates every `CliError` variant and asserts `to_envelope()` produces a well-formed `JsonErr` (non-empty `message`, `code == self.code()`, `retryable` boolean valid, `context` is `Value::Null` or an object — never a scalar).

This would catch, e.g., a future refactor that accidentally returns `context = Value::String(...)` for a variant, or forgets to wire `code()` for a newly added variant.

**Suggested test.** In `src/core/error.rs::tests`, add:

```rust
#[test]
fn every_variant_envelope_round_trips() {
    let samples: Vec<CliError> = vec![
        CliError::OutcomeIncomplete { message: "m".into(), hint: None },
        CliError::OutcomeNotRatified,
        CliError::PlanInvalid { message: "m".into(), hint: None },
        CliError::DagCycle { message: "m".into() },
        CliError::DagMissingDep { message: "m".into() },
        CliError::TaskNotReady { message: "m".into() },
        CliError::ProofMissing { path: "p".into() },
        CliError::ReviewFindingsBlock { message: "m".into() },
        CliError::ReplanRequired { message: "m".into() },
        CliError::CloseNotReady { message: "m".into() },
        CliError::StateCorrupt { message: "m".into() },
        CliError::RevisionConflict { expected: 1, actual: 2 },
        CliError::StaleReviewRecord { message: "m".into() },
        CliError::TerminalAlreadyComplete { closed_at: "t".into() },
        CliError::ConfigMissing { message: "m".into() },
        CliError::MissionNotFound { message: "m".into(), hint: None },
        CliError::ParseError { message: "m".into() },
        CliError::NotImplemented { command: "c".into() },
    ];
    for err in samples {
        let env = err.to_envelope();
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["ok"], false, "variant {}: ok must be false", err.code());
        assert_eq!(json["code"], err.code());
        assert!(json["message"].as_str().is_some_and(|m| !m.is_empty()),
            "variant {}: message must be non-empty", err.code());
        // context is either absent (Null) or an object — never a scalar.
        let ctx = json.get("context");
        assert!(ctx.is_none() || ctx.unwrap().is_object(),
            "variant {}: context must be object-or-absent, got {:?}",
            err.code(), ctx);
    }
}
```

Severity P3 because every variant already has at least one integration test exercising its `code`, and the round-1 unit tests cover the reserved variants. This is a belt-and-suspenders guard for future refactors.

### P3-2 · Optional Verdict/status agreement via proptest (already noted and rejected in round 1)

Round 1 rejected test-adequacy P3-1 "proptest-style agreement test" as out-of-scope. I concur — the enumerated matrix in `status_close_agreement.rs` + `close.rs` is sufficient for the current verdict set. Re-surfacing only to close the loop: no action needed.
