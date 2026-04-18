---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: mission_contract_kernel
version: 1
spec_revision: 1
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

Establish one native mission-contract center of gravity for mission legality and projections.

## In Scope

- Introduce normalized kernel types and derivation logic in `codex1-core` for canonical mission truth.
- Recast closeout, active-cycle, resume, and cached-state logic onto the kernel or thin projections over it.
- Make validator and internal command paths consume the same legality model instead of reconstructing it independently.
- Add regression coverage for legal transitions, identity stability, projection parity, and no-false-terminal behavior.

## Out Of Scope

- Full helper-surface transaction engine work.
- Full artifact-registry generation work.
- Public skill UX rewrite beyond what is required to preserve the touched kernel contracts.

## Dependencies

- Locked umbrella mission truth in `OUTCOME-LOCK.md`.
- Existing deterministic internal command surface remains the compatibility boundary.
- Planning blueprint revision 1 is the governing route contract.

## Touched Surfaces

- Core mission-state types and runtime derivation paths.
- Resume-resolution and validator logic.
- Runtime tests covering closeout, projection, and resume behavior.
- Local spec support files for receipts and review context.

## Read Scope

- crates/codex1-core/src
- crates/codex1/src/internal
- crates/codex1/tests
- plans/contract-centered-architecture/specs/mission_contract_kernel

## Write Scope

- crates/codex1-core/src
- crates/codex1/src/internal
- crates/codex1/tests
- plans/contract-centered-architecture/specs/mission_contract_kernel

## Interfaces And Contracts Touched

- Ralph state and closeout contract.
- Resume-resolution contract.
- Mission-artifact validation contract.
- Internal command machine-truth contract.

## Implementation Shape

Add a native kernel and keep current file shapes as compatibility-preserving projections. The slice should centralize legality and derivation first, then let existing readers and writers thin out around that authority instead of adding another wrapper layer.

## Proof-Of-Completion Expectations

- cargo test -p codex1-core
- cargo test -p codex1 runtime_internal
- codex1 internal validate-mission-artifacts --mission-id contract-centered-architecture
- touched legality transitions have explicit regression coverage for no-false-terminal and projection parity behavior

## Non-Breakage Expectations

- Existing internal command entrypoints keep working.
- Manual and autopilot progression do not diverge because of the new kernel.
- Cached state and projection updates remain deterministic and honest.

## Review Lenses

- correctness
- evidence_adequacy
- interface_compatibility
- protected_surface_integrity

## Replan Boundary

- Reopen planning if a single native kernel cannot preserve the required mission semantics without weakening the native-Codex product contract.
- Reopen planning if the slice forces a second hidden authority path instead of reducing authority duplication.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- docs/codex1-prd.md

## Freshness Notes

- Current for lock revision 1 and the present `runtime.rs` plus `ralph.rs` architecture.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
