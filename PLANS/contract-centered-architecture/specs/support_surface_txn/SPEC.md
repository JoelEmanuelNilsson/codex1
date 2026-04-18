---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: support_surface_txn
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

Make helper-surface mutation transactional, reversible, and honest.

## In Scope

- Move backup manifest ownership into one shared core transaction model.
- Add explicit staging, commit, rollback, and crash-recovery semantics for support-surface mutations.
- Make setup, restore, and uninstall thin views over the shared engine.
- Strengthen doctor so it reasons from committed truth and drift, not sibling command-specific assumptions.

## Out Of Scope

- Mission-state kernel work outside the support-surface boundary.
- Qualification evidence redesign except where support-surface artifacts become transaction-driven.
- Public skill behavior changes.

## Dependencies

- The mission-contract kernel should already be in place so support-surface truth can align with the same contract-centered design posture.
- Planning blueprint revision 1 remains the governing route truth.

## Touched Surfaces

- Core backup and manifest model.
- Setup, restore, uninstall, and doctor command implementations.
- Qualification and command tests for helper-surface behavior.

## Read Scope

- crates/codex1-core/src
- crates/codex1/src/commands
- crates/codex1/tests
- plans/contract-centered-architecture/specs/support_surface_txn

## Write Scope

- crates/codex1-core/src
- crates/codex1/src/commands
- crates/codex1/tests
- plans/contract-centered-architecture/specs/support_surface_txn

## Interfaces And Contracts Touched

- Support-surface backup and restore contract.
- Doctor honesty contract.
- Qualification helper-surface gates.

## Implementation Shape

Extract one shared transaction engine in core, then rewrite the command surfaces as bounded transaction clients. The manifest should describe committed truth, not act as an in-flight imperative control channel.

## Proof-Of-Completion Expectations

- interrupted or failed helper-surface mutations can be diagnosed and rolled back from journal truth
- setup, restore, and uninstall stop duplicating manifest schema and mutation semantics
- helper qualification gates still pass or become stronger
- cargo test -p codex1 qualification_cli

## Non-Breakage Expectations

- existing helper command names and basic user-facing responsibilities remain intact
- supported-surface backups stay reversible and explicit
- doctor remains honest about unsupported or drifted environments

## Review Lenses

- correctness
- operability_rollback_observability
- evidence_adequacy

## Replan Boundary

- Reopen planning if the transaction engine would force a helper-first product model or blur source-repo versus target-repo responsibilities.
- Reopen planning if rollback honesty cannot be preserved without a materially different support-surface contract.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- docs/runtime-backend.md

## Freshness Notes

- Current for lock revision 1 and the present setup, restore, and uninstall command split.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
