---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: artifact_contract_registry
version: 1
spec_revision: 2
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

Create one machine-readable artifact-requirements registry for validators and scaffolds.

## In Scope

- Design and implement a typed artifact-requirements registry.
- Move README, review-ledger, replan-log, and other visible-artifact validation away from marker checks toward contract-aware validation.
- Reuse the registry to drive or tighten template scaffolding and parity checks.
- Preserve human-readable artifacts as the product surface while reducing prose-only duplication.

## Out Of Scope

- Replacing markdown artifacts with a hidden machine-only format.
- Broad workflow-surface changes not required to adopt the registry.
- Support-surface transaction mechanics.

## Dependencies

- The mission-contract kernel must define stable governing truth and projection concepts first.
- Planning blueprint revision 1 remains the governing route truth.

## Touched Surfaces

- Artifact typing and validation logic.
- Mission templates and visible-artifact parity checks.
- Internal validator commands and any helper utilities that consume artifact requirements.

## Read Scope

- crates/codex1-core/src
- crates/codex1/src/internal
- templates/mission
- plans/contract-centered-architecture/specs/artifact_contract_registry

## Write Scope

- crates/codex1-core/src
- crates/codex1/src/internal
- templates/mission
- plans/contract-centered-architecture/specs/artifact_contract_registry

## Interfaces And Contracts Touched

- Visible artifact contract registry.
- Mission artifact validator contract.
- Template and scaffold parity contract.

## Implementation Shape

Create one typed registry in core, then make validators and template consumers read from it. Keep the artifacts human-readable and explicit; centralize the contract, not the presentation.

## Proof-Of-Completion Expectations

- visible-artifact validation fails for structurally weak artifacts that previously passed marker checks
- templates and validators stay in parity for the moved artifact classes
- cargo test -p codex1-core
- cargo test -p codex1 runtime_internal

## Non-Breakage Expectations

- Existing visible artifact paths remain canonical.
- README, ledger, and log generation stays human-readable.
- The registry does not become a hidden second planning engine.

## Review Lenses

- correctness
- evidence_adequacy
- interface_compatibility

## Replan Boundary

- Reopen planning if the registry approach cannot preserve the explicit visible-artifact product stance.
- Reopen planning if template generation would require hiding canonical truth in non-user-facing machinery.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- docs/codex1-prd.md

## Freshness Notes

- Current for lock revision 1 and the present artifact, validator, and template split.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
