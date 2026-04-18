---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: reviewer_parent_writeback_guard
version: 1
spec_revision: 2
artifact_status: active
packetization_status: runnable
execution_status: packaged
owner_mode: solo
blueprint_revision: 16
blueprint_fingerprint: sha256:cded0d54c8006846633e31bb172ffa0ba686ccbc69a147d8b1cdca40528173ca
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

Prevent reviewer-lane self-writeback after bounded reviewer output failures by enforcing a parent-only review writeback boundary in runtime and public workflow docs.

## In Scope

- Add runtime rejection for reviewer-lane-like `record-review-outcome` callers, including reviewer identities or evidence patterns that indicate the child reviewer is attempting to clear its own gate.
- Add regression tests proving a reviewer lane cannot record a clean review outcome even when it cites `reviewer-output:<lane>` evidence.
- Preserve parent/orchestrator writeback for contaminated waves and legitimate parent-owned clean/failed review dispositions.
- Update `$review-loop`, internal orchestration docs, and runtime docs to describe the result-capture boundary.
- Record proof receipts for the guard and leave qualification review to rerun after this guard passes review.

## Out Of Scope

- Removing reviewer-agent judgment as the source of substantive review truth.
- Introducing a wrapper runtime or external babysitter outside native Codex.
- Malicious filesystem sandboxing beyond the observed writeback path.
- Accepting the contaminated clean review from bundle 71f23706 as proof.

## Dependencies

- reviewer_lane_canonical_write_isolation is complete and review-clean.
- Contradiction b91d1f83 is accepted for blueprint replan.
- Existing parent-held review truth snapshot guard remains intact.

## Touched Surfaces

- crates/codex1-core/src/runtime.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/reviewer_parent_writeback_guard

## Read Scope

- crates/codex1-core/src
- crates/codex1/tests
- .codex/skills
- docs
- PLANS/review-loop-delegated-review-only
- .ralph/missions/review-loop-delegated-review-only

## Write Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/reviewer_parent_writeback_guard

## Interfaces And Contracts Touched

- `record-review-outcome` authority validation.
- Reviewer output evidence reference semantics.
- Parent-owned review-loop writeback contract.
- Review-loop public skill and internal orchestration instructions.

## Implementation Shape

Introduce a narrow runtime predicate that detects reviewer-lane-like writeback attempts from reviewer identity and evidence-ref shape. `record-review-outcome` must reject clean or blocking review dispositions when the caller identity looks like a child reviewer lane self-clearing its own bundle, even if it cites `reviewer-output:<lane>` evidence. Parent/orchestrator callers can still record outcomes when they provide the parent-held review truth snapshot and reviewer-agent output evidence, or when they route contaminated waves to replan with contamination evidence.

## Proof-Of-Completion Expectations

- cargo test -p codex1 --test runtime_internal reviewer_parent_writeback_guard --quiet
- cargo test -p codex1 --test runtime_internal delegated_review_authority --quiet
- cargo test -p codex1 --test runtime_internal reviewer_lane_canonical_write_isolation --quiet
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id review-loop-delegated-review-only

## Non-Breakage Expectations

- Parent-owned clean review writeback with reviewer output evidence still passes.
- Contaminated-wave parent writeback to replan still passes.
- Child evidence snapshots remain redacted and validated by guard metadata.
- No mission-close path accepts the contaminated 71f23706 review as clean proof.

## Review Lenses

- correctness
- evidence_adequacy
- operability_rollback_observability
- interface_compatibility

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

- PLANS/review-loop-delegated-review-only/OUTCOME-LOCK.md
- PLANS/review-loop-delegated-review-only/PROGRAM-BLUEPRINT.md
- .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:b91d1f83-8db2-4c91-9bc9-711ab37a1f70
- PLANS/review-loop-delegated-review-only/REPLAN-LOG.md

## Freshness Notes

- Refresh if native Codex exposes hard read-only reviewer roles or explicit reviewer-lane capability metadata.
- Refresh if review-output evidence refs gain a durable artifact-backed result schema.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
