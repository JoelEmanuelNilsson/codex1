---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: delegated_review_qualification_guard
version: 1
spec_revision: 6
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

Add qualification and documentation regression checks for the delegated-review-only contract. This guard makes sure future $review-loop changes cannot quietly reintroduce parent self-review.

## In Scope

- Add qualification coverage that checks $review-loop and backend docs forbid parent self-review and require reviewer-agent judgment.
- Add qualification or runtime test coverage for the review outcome evidence requirement introduced by delegated_review_authority_contract.
- Repair execution-package dependency binding so `spec:<id>` graph dependencies can be expressed as revision-bearing package dependency rows without weakening dependency truth.
- Update qualification docs and gates to describe the parent-orchestrator versus reviewer-agent authority split.
- Record proof receipts for the targeted qualification gate.

## Out Of Scope

- New reviewer profiles or model-routing changes.
- Any change that permits parent self-review as a fallback.
- Platform-level sandbox enforcement.

## Dependencies

- delegated_review_authority_contract complete and review-clean.
- Stable evidence-ref shape for reviewer-agent outputs.

## Touched Surfaces

- crates/codex1/src/commands/qualify.rs
- crates/codex1-core/src/runtime.rs
- crates/codex1/tests/qualification_cli.rs
- docs/qualification/README.md
- docs/qualification/gates.md
- .codex/skills/review-loop/SKILL.md
- docs/runtime-backend.md
- PLANS/review-loop-delegated-review-only/specs/delegated_review_qualification_guard

## Read Scope

- crates/codex1/src/commands
- crates/codex1-core/src
- crates/codex1/tests
- docs/qualification
- .codex/skills/review-loop/SKILL.md
- docs/runtime-backend.md

## Write Scope

- crates/codex1/src/commands/qualify.rs
- crates/codex1-core/src/runtime.rs
- crates/codex1/tests/qualification_cli.rs
- docs/qualification/README.md
- docs/qualification/gates.md
- PLANS/review-loop-delegated-review-only/specs/delegated_review_qualification_guard

## Interfaces And Contracts Touched

- Qualification gate registry.
- Execution-package dependency binding for `spec:<id>` graph dependencies.
- Public qualification documentation.
- Review-loop authority regression checks.

## Implementation Shape

Add a focused qualification gate or extend the existing reviewer capability/review-loop qualification coverage so it inspects public skills/docs and the runtime outcome authority behavior. The gate should fail if docs imply parent local review is acceptable, if child reviewers are asked to invoke skills or write truth, or if review outcomes can pass without reviewer-agent evidence under the new contract.

Keep the package dependency contract aligned with the graph contract: when a graph node depends on another spec, `spec:<id>` dependency rows must be accepted as governing, fingerprinted dependency truth rather than forcing callers to choose between an unsatisfied graph dependency and an unbound dependency row.

## Proof-Of-Completion Expectations

- cargo test -p codex1-core compile_execution_package_accepts_spec_dependency_governing_refs --quiet
- Targeted qualification test passes and fails for representative missing delegated-review-only phrases or missing outcome evidence behavior.
- cargo test -p codex1 --test qualification_cli delegated_review --quiet or equivalent passes.
- cargo run -p codex1 -- qualify-codex delegated_review --json or equivalent targeted gate produces non-zero checks.
- Qualification docs name the gate and explain what it protects.

## Non-Breakage Expectations

- Existing qualification gates remain green.
- Existing reviewer capability boundary gate remains green.
- Public docs stay skills-first and do not introduce wrapper-runtime requirements.

## Review Lenses

- release_gate_integrity
- evidence_adequacy
- spec_conformance
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
- PLANS/review-loop-delegated-review-only/specs/delegated_review_authority_contract/SPEC.md
- docs/qualification/README.md

## Freshness Notes

- Depends on the authority evidence shape selected in the first spec.
- Contaminated clean review 71f23706 is invalidated by contradiction b91d1f83 and must not satisfy mission closure.
- Should be refreshed if qualification gate naming, command routing, or reviewer writeback authorization changes.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
