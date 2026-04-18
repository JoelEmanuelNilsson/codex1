---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: reviewer_output_inbox_contract
version: 1
spec_revision: 1
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

Add a bounded child-writeable reviewer output inbox so reviewer lanes can return NONE/findings without mutating review gates, closeouts, ledgers, specs, or mission-close artifacts.

## In Scope

- Add a reviewer-output artifact type and internal command that records only bounded reviewer outputs (`NONE` or structured findings) for one review bundle.
- Store reviewer outputs outside gate/closeout/spec completion mutation paths.
- Require parent `record-review-outcome` evidence refs to cite existing reviewer-output inbox artifacts, not arbitrary `reviewer-output:<lane>` strings.
- Add validation that reviewer-output artifacts bind mission id, bundle id, reviewer id, output shape, severity, evidence refs, and source evidence snapshot.
- Update `$review-loop`, internal orchestration, and runtime docs to route child reviewers through the inbox.

## Out Of Scope

- Letting child reviewers clear gates, write review ledgers, compile packages, or decide completion.
- Malicious filesystem sandboxing beyond the bounded inbox command.
- Accepting contaminated gates `55acf452`, `de12d0dd`, or closeouts `57`, `64` as clean proof.

## Dependencies

- reviewer_lane_canonical_write_isolation complete and review-clean.
- review_writeback_authority_token implementation may be preserved but must be revalidated after inbox lands.
- Contradiction ed548fc6 is accepted for blueprint replan.

## Touched Surfaces

- crates/codex1-core/src/runtime.rs
- crates/codex1-core/src/paths.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/reviewer_output_inbox_contract

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
- crates/codex1-core/src/paths.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/reviewer_output_inbox_contract

## Interfaces And Contracts Touched

- Reviewer output artifact schema.
- Internal reviewer-output recording command.
- `record-review-outcome` evidence validation.
- Review-loop child output routing contract.

## Implementation Shape

Introduce a narrow append-only reviewer output inbox under hidden Ralph mission state. Child reviewers may call only the reviewer-output command to persist their bounded result for a specific bundle and evidence snapshot. The command must not update gates, closeouts, ledgers, specs, packages, or mission completion. Parent review writeback must cite these artifacts and validate their bundle/snapshot bindings before recording review outcomes.

## Proof-Of-Completion Expectations

- cargo test -p codex1 --test runtime_internal reviewer_output_inbox_contract --quiet
- cargo test -p codex1 --test runtime_internal review_writeback_authority_token --quiet
- cargo test -p codex1 --test runtime_internal delegated_review_authority --quiet
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id review-loop-delegated-review-only

## Non-Breakage Expectations

- Reviewer output recording cannot mutate gates, closeouts, ledgers, specs, packages, or state completion.
- Parent writeback with valid inbox output remains possible.
- Parent writeback with arbitrary reviewer-output strings is rejected after the inbox is enabled.
- Existing token and redaction tests remain green.

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

- .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:ed548fc6-c0bd-4a33-9e4e-b51fd398dc63
- PLANS/review-loop-delegated-review-only/REPLAN-LOG.md
- PLANS/review-loop-delegated-review-only/PROGRAM-BLUEPRINT.md

## Freshness Notes

- Refresh if native Codex exposes hard read-only reviewer roles or structured child result APIs.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
