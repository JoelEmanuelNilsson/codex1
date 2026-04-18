---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: qualification_evidence_pipeline
version: 1
spec_revision: 3
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 10
blueprint_fingerprint: sha256:0e73f4340105227393ea58ff06a5ef2e5784a282c52f4b7d420d5f71bca87a3d
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

Make qualification evidence-first and raw-artifact judged.

## In Scope

- Persist raw evidence for live stop-hook, exec-resume, and child-lane qualification flows.
- Split probe execution from assessment logic where they are currently tangled.
- Make reports point back to the raw artifacts used for judgment.
- Tighten qualification proofs for native child-lane reconciliation, waiting identity, and manual or autopilot parity.

## Out Of Scope

- Replacing qualification with a different operator product.
- Broad support-surface transaction redesign beyond what the evidence pipeline reads.
- Public skill UX changes except where proof requirements need better surfaced artifacts.

## Dependencies

- The mission-contract kernel and artifact registry should already reduce authority duplication in the flows being qualified.
- Planning blueprint revision 1 remains the governing route truth.

## Touched Surfaces

- Qualification command flows and their evidence layout.
- Qualification docs and report schema.
- Tests for live and non-live qualification behavior.

## Read Scope

- crates/codex1/src/commands
- crates/codex1/tests
- docs/qualification
- plans/contract-centered-architecture/specs/qualification_evidence_pipeline

## Write Scope

- crates/codex1/src/commands
- crates/codex1/tests
- docs/qualification
- plans/contract-centered-architecture/specs/qualification_evidence_pipeline

## Interfaces And Contracts Touched

- Supported-build qualification contract.
- Evidence layout and report schema contract.
- Native multi-agent reconciliation proof contract.

## Implementation Shape

Capture raw artifacts first, then assess those artifacts with deterministic judges. Model-authored summaries may remain as convenience outputs, but not as the decisive proof surface.

## Proof-Of-Completion Expectations

- live qualification reports preserve the raw artifacts needed to justify each pass or fail decision
- native child-lane qualification can distinguish surface gaps from Codex1 reconcile bugs honestly
- manual and autopilot parity and Ralph waiting proofs remain inspectable and stable
- cargo test -p codex1 qualification_cli

## Non-Breakage Expectations

- qualification remains machine-readable and versioned
- doctor can still surface qualification freshness and status honestly
- supported-build qualification remains tied to the exact trusted build under test

## Review Lenses

- correctness
- evidence_adequacy
- operability_rollback_observability

## Replan Boundary

- Reopen planning if evidence-first qualification cannot prove the approved fully autonomous execution contract honestly.
- Reopen planning if the supported-build proof boundary must materially change the umbrella product claim.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- docs/qualification/README.md

## Freshness Notes

- Current for lock revision 1 and the present qualification gate and report contract.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
