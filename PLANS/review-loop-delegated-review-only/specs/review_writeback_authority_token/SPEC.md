---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: review_writeback_authority_token
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

Require ephemeral parent-held writeback authority so child-readable review truth snapshots cannot clear review gates.

## In Scope

- Extend review truth/writeback validation so persisted review truth snapshots alone are not sufficient to record review outcomes.
- Add an ephemeral writeback authority token returned to the parent by capture-review-truth-snapshot; persist only a verifier/hash in repo artifacts.
- Require record-review-outcome to receive and verify the plaintext token before mutating gates, closeouts, ledgers, specs, or mission-close artifacts.
- Ensure child evidence snapshots and persisted review truth files do not expose the plaintext token.
- Add regression tests for parent writeback with token, reviewer/child writeback without token, and parent impersonation using only repo-visible truth files.
- Update review-loop/internal orchestration/runtime docs with the token boundary.

## Out Of Scope

- Removing delegated reviewer judgment.
- Building an external wrapper runtime or daemon.
- Treating the token as malicious-process security if the parent intentionally leaks it to children.
- Accepting contaminated gate 55acf452 or closeout 57 as clean review evidence.

## Dependencies

- reviewer_lane_canonical_write_isolation is complete and review-clean.
- reviewer_parent_writeback_guard implementation may be preserved but must be re-reviewed after this token guard.
- Contradiction 7de544a7 is accepted for blueprint replan.

## Touched Surfaces

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/review_writeback_authority_token

## Read Scope

- crates/codex1-core/src
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests
- .codex/skills
- docs
- PLANS/review-loop-delegated-review-only
- .ralph/missions/review-loop-delegated-review-only

## Write Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/review_writeback_authority_token

## Interfaces And Contracts Touched

- `capture-review-truth-snapshot` output contract.
- `ReviewTruthSnapshot` persisted verifier contract.
- `ReviewResultInput` / `record-review-outcome` writeback authority contract.
- Review-loop public skill and internal orchestration instructions.

## Implementation Shape

Generate a fresh random token when the parent captures review truth. Return the plaintext token only in command output to the parent. Persist only a verifier/hash with the review truth snapshot and include no plaintext token in child evidence snapshots. Require record-review-outcome to receive the plaintext token and verify it against the persisted verifier before any durable review writeback. Reviewer lanes with only repo-visible files must fail writeback even if they spoof parent reviewer identity and cite reviewer-output evidence.

## Proof-Of-Completion Expectations

- cargo test -p codex1 --test runtime_internal review_writeback_authority_token --quiet
- cargo test -p codex1 --test runtime_internal reviewer_parent_writeback_guard --quiet
- cargo test -p codex1 --test runtime_internal delegated_review_authority --quiet
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id review-loop-delegated-review-only

## Non-Breakage Expectations

- Parent-owned contaminated-wave replan writeback still works when the parent has the token.
- Parent-owned clean writeback still works when reviewer output evidence and token are present.
- Existing review evidence snapshot redaction remains intact.
- Existing qualification proof/package work remains preserved pending fresh review.

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
- .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:7de544a7-a8ae-444f-8b7d-396de9b5344c
- PLANS/review-loop-delegated-review-only/REPLAN-LOG.md

## Freshness Notes

- Refresh if native Codex exposes hard read-only reviewer roles or parent-only secret storage.
- Refresh if review result capture moves to a backend-owned artifact store.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
