---
artifact: workstream-spec
mission_id: reviewer-lane-capability-boundary
spec_id: reviewer_evidence_snapshot_contract
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

Define frozen reviewer evidence snapshots/briefs so child reviewers can judge bounded work without being handed live mutable mission-truth paths by default.

## In Scope

- Add a reviewer evidence snapshot or brief artifact that captures the review bundle, relevant source excerpts/diffs, receipts, and governing fingerprints.
- Update `$review-loop` and internal orchestration docs to prefer snapshot refs over live mutable repo paths for child lanes.
- Define when live repo reads are still allowed and how parent must guard them.

## Out Of Scope

- Replacing human-quality review judgment with static validation.
- Removing parent-owned review outcome recording.

## Dependencies

- `reviewer_lane_mutation_guard`

## Touched Surfaces

- `crates/codex1-core/src/runtime.rs`
- `crates/codex1/src/internal/mod.rs`
- `.codex/skills/review-loop/SKILL.md`
- `.codex/skills/internal-orchestration/SKILL.md`
- `docs/runtime-backend.md`

## Read Scope

- crates/codex1-core/src
- crates/codex1/src
- .codex/skills
- docs

## Write Scope

- crates/codex1-core/src
- crates/codex1/src
- .codex/skills
- docs
- PLANS/reviewer-lane-capability-boundary/specs/reviewer_evidence_snapshot_contract

## Interfaces And Contracts Touched

- Review bundle consumption contract.
- Child reviewer brief contract.
- Evidence adequacy for findings-only lanes.

## Implementation Shape

Create a deterministic evidence snapshot contract that packages enough read-only context for reviewer lanes while keeping mission truth writeback parent-owned and mutation-guarded.

## Proof-Of-Completion Expectations

- Snapshot artifact includes bundle id, source package id, governing fingerprints, proof rows, receipts, changed-file context, and review instructions.
- Validation rejects snapshots that omit required governing refs or evidence rows.
- Docs/skills route child reviewers to snapshot refs first and live paths only under the mutation guard.

## Non-Breakage Expectations

- Existing review bundles remain valid.
- Reviewers can still report precise evidence refs.

## Review Lenses

- spec_conformance
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
- `.codex/skills/review-loop/SKILL.md`

## Freshness Notes

- Runs after mutation guard stabilizes.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
