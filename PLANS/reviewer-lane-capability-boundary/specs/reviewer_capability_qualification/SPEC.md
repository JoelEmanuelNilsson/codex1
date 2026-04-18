---
artifact: workstream-spec
mission_id: reviewer-lane-capability-boundary
spec_id: reviewer_capability_qualification
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 6
blueprint_fingerprint: sha256:e96b3bbfc2372c1d3cd07d61853a9de922f9152549d1e96a2bedb7ce69cd60a2
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

Make the reviewer capability boundary part of Codex1 qualification so this failure mode cannot regress silently.

## In Scope

- Add qualification coverage for contaminated child review waves.
- Add qualification coverage for clean parent-owned review writeback after unchanged child outputs.
- Update qualification docs/gates with the new release-blocking evidence.

## Out Of Scope

- Broader qualification redesign.
- Non-review-lane autonomy changes.

## Dependencies

- `reviewer_lane_mutation_guard`
- `reviewer_evidence_snapshot_contract`

## Touched Surfaces

- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1/tests/qualification_cli.rs`
- `docs/qualification/README.md`
- `docs/qualification/gates.md`

## Read Scope

- crates/codex1/src/commands
- crates/codex1/tests
- docs/qualification

## Write Scope

- crates/codex1/src/commands
- crates/codex1/tests
- docs/qualification
- PLANS/reviewer-lane-capability-boundary/specs/reviewer_capability_qualification

## Interfaces And Contracts Touched

- Qualification evidence model.
- Release gate for reviewer-lane isolation.

## Implementation Shape

Extend qualification to exercise the new guard and evidence snapshot route, persisting raw command output or receipts so proof is not just a summary.

## Proof-Of-Completion Expectations

- Qualification command fails or records a blocker if contaminated review wave is accepted.
- Qualification command proves clean parent writeback still works.
- Docs name the gate as release-blocking.

## Non-Breakage Expectations

- Existing qualification suites remain green.
- No weakening of mission-close review qualification.

## Review Lenses

- correctness
- evidence_adequacy
- release_gate_integrity

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
- `docs/qualification/README.md`

## Freshness Notes

- Runs after runtime and reviewer evidence contracts stabilize.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
