---
artifact: workstream-spec
mission_id: review-lane-role-contract
spec_id: ralph_review_lane_isolation
version: 1
spec_revision: 2
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 10
blueprint_fingerprint: sha256:73643e5ae5a1c12e2c80c3b51aafda42fd133e8eb835b7c9d1e19d72be9bd665
spec_fingerprint: null
replan_boundary:
  local_repair_allowed: false
  trigger_matrix:
  - trigger_code: write_scope_expansion
    reopen_layer: execution_package
  - trigger_code: interface_contract_change
    reopen_layer: blueprint
  - trigger_code: dependency_truth_change
    reopen_layer: execution_package
  - trigger_code: proof_obligation_change
    reopen_layer: blueprint
  - trigger_code: review_contract_change
    reopen_layer: blueprint
  - trigger_code: protected_surface_change
    reopen_layer: mission_lock
  - trigger_code: migration_rollout_change
    reopen_layer: blueprint
  - trigger_code: outcome_lock_change
    reopen_layer: mission_lock
---
# Workstream Spec

## Purpose

Make child review lanes Ralph-safe and parent-writeback-only.

## In Scope

- Add/refine lane-role metadata for child review lanes.
- Distinguish parent controller gates from child reviewer result delivery.
- Ensure parent-only review writeback is authoritative.
- Add tests for parent blocked state plus child reviewer completion/result delivery.

## Out Of Scope

- External wrapper runtimes.
- Child reviewer mission-truth mutation.
- Full review-loop orchestration.

## Dependencies

- `reviewer_profile_contracts`.

## Touched Surfaces

- `crates/codex1-core/src/ralph.rs`
- `crates/codex1-core/src/runtime.rs`
- `crates/codex1/tests/runtime_internal.rs`
- `docs/MULTI-AGENT-V2-GUIDE.md`

## Read Scope

- crates/codex1-core/src
- crates/codex1/tests
- docs

## Write Scope

- crates/codex1-core/src
- crates/codex1/tests
- docs
- PLANS/review-lane-role-contract/specs/ralph_review_lane_isolation

## Interfaces And Contracts Touched

- Ralph stop-hook output semantics.
- Active child lane expectations.
- Parent-only review outcome authority.

## Implementation Shape

Introduce the smallest runtime contract that proves child reviewers are bounded findings lanes rather than mission controllers.

## Proof-Of-Completion Expectations

- Child review lane can return findings while parent mission has an open gate.
- Parent gates still block parent mission progress.
- Child outputs do not directly update gates, ledgers, closeouts, or terminal state.

## Non-Breakage Expectations

- Existing waiting, selection, active-cycle, and child-lane qualification remain green.

## Review Lenses

- correctness
- operability_rollback_observability
- evidence_adequacy

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| write_scope_expansion | execution_package |
| interface_contract_change | blueprint |
| dependency_truth_change | execution_package |
| proof_obligation_change | blueprint |
| review_contract_change | blueprint |
| protected_surface_change | mission_lock |
| migration_rollout_change | blueprint |
| outcome_lock_change | mission_lock |

## Truth Basis Refs

- `OUTCOME-LOCK.md`
- `docs/MULTI-AGENT-V2-GUIDE.md`

## Freshness Notes

- Current for the implemented findings-only reviewer lane role and Stop-hook
  behavior.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
