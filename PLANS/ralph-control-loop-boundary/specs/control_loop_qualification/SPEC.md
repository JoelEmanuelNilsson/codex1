---
artifact: workstream-spec
mission_id: ralph-control-loop-boundary
spec_id: control_loop_qualification
version: 1
spec_revision: 7
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 16
blueprint_fingerprint: sha256:2b2cb026e80b79e6dbe64eb59ad439d0fc8b42b784f76617fef2e6ca48e790a8
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

Add qualification and support proof that scoped Ralph control works with hooks installed and does not regress setup, restore, uninstall, or existing qualification expectations.

## In Scope

- Add qualification gates or smoke steps for no-lease user interaction yield, generic subagent yield, active parent lease blocking, and pause/close yield.
- Update qualification docs to explain the scoped control-loop contract.
- Ensure setup/restore/uninstall docs or checks account for hook installation plus control-state escape without requiring manual hook moves.
- Rerun representative qualification CLI tests.

## Out Of Scope

- Implementing runtime lease behavior or skill UX if earlier specs are incomplete.
- Broader support-surface transaction redesign.

## Dependencies

- ralph_loop_lease_runtime complete and reviewed.
- loop_skill_surface_and_pause complete and reviewed.

## Touched Surfaces

- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/qualification_cli.rs
- docs/qualification/README.md
- docs/qualification/gates.md
- docs/runtime-backend.md
- crates/codex1/tests/runtime_internal.rs

## Read Scope

- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/qualification_cli.rs
- docs/qualification
- docs/runtime-backend.md
- PLANS/ralph-control-loop-boundary

## Write Scope

- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/qualification_cli.rs
- docs/qualification/README.md
- docs/qualification/gates.md
- docs/runtime-backend.md
- PLANS/ralph-control-loop-boundary/specs/control_loop_qualification

## Interfaces And Contracts Touched

- Qualification gate registry.
- Qualification report semantics.
- Hook support documentation.

## Implementation Shape

Extend qualification to exercise the same stop-hook control contract expected by users: no active lease yields, subagent payload yields, active parent lease blocks, pause/close yields again, and hook installation remains authoritative. Documentation should explain that hook presence is safe because enforcement is lease-scoped.

## Proof-Of-Completion Expectations

- cargo test -p codex1 --test qualification_cli control_loop --quiet or equivalent targeted qualification test.
- cargo run -p codex1 -- qualify-codex --json --live=false --self-hosting=false produces non-zero checks for the scoped control-loop gate.
- If the source workspace hook is intentionally paused, the source qualification
  run may fail only `project_hooks_file_present`; the receipt must cite the full
  JSON report and the `control_loop_boundary` gate evidence proving the scoped
  gate passed under an installed-hook sandbox.
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id ralph-control-loop-boundary

## Non-Breakage Expectations

- Existing waiting stop-hook qualification remains valid.
- Setup still installs exactly one authoritative Stop pipeline when enabled.
- Manual hook movement is no longer the documented normal escape path.

## Review Lenses

- evidence_adequacy
- release_gate_integrity
- operability_rollback_observability
- interface_compatibility

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| qualification_surface_requires_runtime_api_change | execution_package |
| support_surface_hook_policy_change | blueprint |
| proof_obligation_change | blueprint |
| release_gate_scope_change | blueprint |

## Truth Basis Refs

- PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md
- docs/qualification/README.md
- crates/codex1/src/commands/qualify.rs


## Freshness Notes

- Refresh this spec if native Codex Stop-hook payloads add or remove reliable parent/subagent/user-turn metadata.
- Refresh this spec if the public skill loop entry model changes before execution.

## Support Files

- REVIEW.md records delegated review outcomes for this spec.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output and command transcripts.
