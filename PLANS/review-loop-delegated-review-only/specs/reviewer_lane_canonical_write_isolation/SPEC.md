---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: reviewer_lane_canonical_write_isolation
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
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

Prevent findings-only reviewer lanes from mutating canonical mission truth during review-loop orchestration. This slice makes review child lanes consume review evidence only, while the parent keeps the review truth snapshot as the writeback capability.

## In Scope

- Change review evidence snapshots so child reviewer briefs do not embed the full review truth snapshot required for record-review-outcome.
- Preserve parent-owned review truth snapshot validation by keeping capture-review-truth-snapshot as the parent-held writeback guard.
- Update validate-review-evidence-snapshot to require non-capability guard binding metadata and findings-only instructions instead of exporting the parent writeback snapshot.
- Update $review-loop and internal orchestration guidance so blocking reviewer lanes use explorer-style read-only roles with fork_turns none and frozen evidence snapshots only.
- Add tests proving evidence snapshots no longer contain the writeback snapshot, record-review-outcome still requires a parent-supplied truth snapshot, and public reviewer spawn protocol forbids default/worker mutation-capable review lanes.

## Out Of Scope

- Platform-level filesystem sandboxing.
- Giving child reviewers writeback authority.
- Changing the delegated-review outcome evidence requirement implemented by delegated_review_authority_contract.
- Re-running mission-close review in this slice.

## Dependencies

- Outcome Lock revision 1 for review-loop-delegated-review-only.
- Contradiction 569eac4f-7de8-49f3-b5ef-84ea84eec741.
- Preserved delegated_review_authority_contract implementation and receipt.

## Touched Surfaces

- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- docs/MULTI-AGENT-V2-GUIDE.md
- crates/codex1-core/src/runtime.rs
- crates/codex1-core/src/lib.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- PLANS/review-loop-delegated-review-only/specs/reviewer_lane_canonical_write_isolation

## Read Scope

- .codex/skills
- docs
- crates/codex1-core/src
- crates/codex1/src
- crates/codex1/tests
- PLANS/review-loop-delegated-review-only
- .ralph/missions/review-loop-delegated-review-only

## Write Scope

- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- docs/MULTI-AGENT-V2-GUIDE.md
- crates/codex1-core/src/runtime.rs
- crates/codex1-core/src/lib.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- PLANS/review-loop-delegated-review-only/specs/reviewer_lane_canonical_write_isolation

## Interfaces And Contracts Touched

- ReviewEvidenceSnapshot schema and validation contract.
- capture-review-evidence-snapshot output contract.
- capture-review-truth-snapshot parent writeback capability contract.
- $review-loop child reviewer spawn protocol.
- record-review-outcome guarded writeback preconditions.

## Implementation Shape

Keep review truth snapshots parent-held. capture-review-truth-snapshot continues to return the full ReviewTruthSnapshot for the parent to submit to record-review-outcome. capture-review-evidence-snapshot should no longer serialize that full snapshot into the child-visible evidence snapshot. Instead it should include a non-capability guard binding such as the bundle id, captured-at timestamp, and/or fingerprint summary sufficient for validation and audit, but insufficient for a child lane to call record-review-outcome. The parent review-loop must pass reviewer children only the frozen evidence snapshot, spawn them as explorer-style findings-only lanes with fork_turns none, and never pass the parent writeback snapshot or skill paths as operational instructions. If reviewer outputs are missing or mission truth changes, the parent records contamination/replan rather than clearing review.

## Proof-Of-Completion Expectations

- Test proves capture-review-evidence-snapshot output omits the full review_truth_snapshot field while preserving guard binding metadata and required review context.
- Test proves validate-review-evidence-snapshot still rejects weak reviewer briefs and accepts the new non-capability evidence snapshot shape.
- Test proves record-review-outcome still rejects writeback without a parent-supplied review_truth_snapshot.
- Test or source assertion proves $review-loop reviewer lanes must use explorer-style findings-only spawn protocol with fork_turns none and must not use default/worker reviewer lanes for blocking review.
- cargo test -p codex1 --test runtime_internal reviewer_lane_canonical_write_isolation --quiet or equivalent passes.
- cargo fmt --all --check passes.

## Non-Breakage Expectations

- Existing delegated_review_authority tests remain green.
- Existing reviewer lane mutation guard tests remain green.
- Existing review evidence snapshot validation remains meaningful after schema change.
- Existing review-loop severity, six-loop cap, and parent-owned writeback semantics remain intact.

## Review Lenses

- correctness
- spec_conformance
- evidence_adequacy
- interface_compatibility
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

- PLANS/review-loop-delegated-review-only/OUTCOME-LOCK.md
- .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:569eac4f-7de8-49f3-b5ef-84ea84eec741
- PLANS/review-loop-delegated-review-only/REPLAN-LOG.md
- .codex/skills/review-loop/SKILL.md

## Freshness Notes

- Current for the contaminated review-loop failure observed on 2026-04-16.
- This spec supersedes the unsafe assumption that child-visible review evidence snapshots may include the parent writeback snapshot.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
