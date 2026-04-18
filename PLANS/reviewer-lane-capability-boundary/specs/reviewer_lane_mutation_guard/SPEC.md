---
artifact: workstream-spec
mission_id: reviewer-lane-capability-boundary
spec_id: reviewer_lane_mutation_guard
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

Add the first practical capability boundary for findings-only reviewer lanes: parent-owned mutation snapshots and review outcome authority checks that reject contaminated review waves.

## In Scope

- Define the mission-truth artifact set that must not change during child reviewer execution: gates, closeouts, state, review ledger, per-spec review files, specs, receipts, bundles, mission-close artifacts, and active-cycle review metadata.
- Add or expose deterministic snapshot/diff helpers around parent review waves, or equivalent validation inside review-loop/record-review-outcome.
- Ensure child-lane Stop-hook bypass only permits result delivery and does not imply writeback authority.
- Add regression tests for a child lane or simulated child phase mutating mission truth before parent aggregation.
- Preserve the clean parent-owned review outcome path.

## Out Of Scope

- Full reviewer evidence export ergonomics.
- Platform-level read-only filesystem sandboxing.
- Changing public skill names or review severity semantics.

## Dependencies

- Outcome Lock revision 1.
- Existing review-lane-role-contract behavior in the current dirty tree.

## Touched Surfaces

- `crates/codex1-core/src/runtime.rs`
- `crates/codex1/src/internal/mod.rs`
- `crates/codex1/tests/runtime_internal.rs`
- `.codex/skills/review-loop/SKILL.md`
- `.codex/skills/internal-orchestration/SKILL.md`
- `docs/runtime-backend.md`
- `docs/MULTI-AGENT-V2-GUIDE.md`

## Read Scope

- crates/codex1-core/src
- crates/codex1/src
- crates/codex1/tests
- .codex/skills
- docs

## Write Scope

- crates/codex1-core/src
- crates/codex1/src
- crates/codex1/tests
- .codex/skills
- docs
- PLANS/reviewer-lane-capability-boundary/specs/reviewer_lane_mutation_guard

## Interfaces And Contracts Touched

- Parent `$review-loop` review-wave authority.
- `record-review-outcome` acceptance contract.
- Stop-hook parent versus findings-only child lane semantics.
- Review gate, closeout, state, and review ledger mutation rules.

## Implementation Shape

Introduce a parent-owned review-wave guard. At review-wave launch, parent records a deterministic snapshot or hash set of mission truth. Before accepting reviewer results and before recording clean/non-clean review outcome, parent validates no watched truth changed outside an allowed parent writeback phase. If drift is detected, return a blocking violation such as `reviewer_lane_truth_mutation_detected` and leave the gate uncleared. If the current architecture cannot attach this directly to native subagent launch, expose deterministic helpers and qualification tests that the public `$review-loop` skill must call.

## Proof-Of-Completion Expectations

- Test proves a simulated child reviewer mutation to gates/closeouts/state/review ledger before parent aggregation is detected and blocks clean review writeback.
- Test proves clean findings-only child output plus unchanged mission truth allows parent-owned `record-review-outcome` to pass the gate.
- Test proves parent Stop-hook still blocks on open review gate while findings-only child Stop-hook may return results.
- `validate-mission-artifacts`, `validate-gates`, and `validate-closeouts` remain green after clean parent-owned review.

## Non-Breakage Expectations

- Existing review-bundle validation remains valid.
- Existing mission-close review behavior remains valid when invoked by parent authority.
- Existing waiting, selection, and active-cycle resume behavior remains green.

## Review Lenses

- correctness
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

- `OUTCOME-LOCK.md`
- `PLANS/review-lane-role-contract/OUTCOME-LOCK.md`
- `.codex/skills/review-loop/SKILL.md`

## Freshness Notes

- Current for the live incident where child review lanes cleared gates and mission-close state.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
