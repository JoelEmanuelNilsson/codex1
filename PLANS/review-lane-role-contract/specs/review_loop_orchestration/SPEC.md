---
artifact: workstream-spec
mission_id: review-lane-role-contract
spec_id: review_loop_orchestration
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

Implement and prove parent `$review-loop` orchestration semantics.

## In Scope

- Select review profiles at proof-worthy boundaries.
- Collect, deduplicate, and aggregate P0/P1/P2 findings.
- Trigger repair, rerun targeted review, and route six non-clean loops to replan.
- Ensure parent records review outcomes after reconciling child outputs.

## Out Of Scope

- Planning-quality redesign beyond this mission.
- External wrapper orchestration.
- Child-owned mission truth.

## Dependencies

- `review_loop_skill_surface`
- `reviewer_profile_contracts`
- `ralph_review_lane_isolation`

## Touched Surfaces

- `.codex/skills/review-loop/SKILL.md`
- `docs/runtime-backend.md`
- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1/tests/runtime_internal.rs`

## Read Scope

- .codex/skills
- docs
- crates/codex1/src/commands
- crates/codex1/tests

## Write Scope

- .codex/skills
- docs
- crates/codex1/src/commands
- crates/codex1/tests
- PLANS/review-lane-role-contract/specs/review_loop_orchestration

## Interfaces And Contracts Touched

- Parent `$review-loop` workflow.
- Review outcome aggregation.
- Repair/review/replan branch discipline.

## Implementation Shape

Keep `$review-loop` as the public orchestration contract and prove clean, repair, and capped-replan paths.

## Proof-Of-Completion Expectations

- Clean loop path records clean outcome and allows continuation or mission close.
- Non-clean path routes to repair and reruns the relevant profile.
- Six consecutive non-clean loops route to replan.

## Non-Breakage Expectations

- Existing review bundle and record-review-outcome contracts remain valid.

## Review Lenses

- spec_conformance
- correctness
- evidence_adequacy
- operability_rollback_observability

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
- `PROGRAM-BLUEPRINT.md`

## Freshness Notes

- Current for the implemented deterministic review-loop decision proof.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
