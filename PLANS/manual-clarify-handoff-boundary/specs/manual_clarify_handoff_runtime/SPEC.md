---
artifact: workstream-spec
mission_id: manual-clarify-handoff-boundary
spec_id: manual_clarify_handoff_runtime
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 1
blueprint_fingerprint: sha256:1bb89621f7c85816dbf8dc0d4138bb0da02e547100ea086685ed6480da26c97c
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

Fix runtime handoff after ratified manual clarify.

## In Scope

- Change locked `init-mission` clarify closeout semantics from actionable planning continuation to durable manual handoff.
- Add runtime tests proving manual clarify Stop does not block with `Start $plan`.
- Add/keep tests or assertions proving `$autopilot` remains the automatic continuation surface.
- Update docs/skill text if needed to make the boundary explicit.

## Out Of Scope

- Full autopilot implementation redesign.
- Planning quality redesign.
- Changing public skill names.

## Dependencies

- Outcome Lock revision 1.
- Current runtime Stop-hook and resume behavior.

## Touched Surfaces

- `crates/codex1-core/src/runtime.rs`
- `crates/codex1/tests/runtime_internal.rs`
- `.codex/skills/clarify/SKILL.md` if wording needs tightening
- `.codex/skills/autopilot/SKILL.md` if wording needs tightening
- `docs/runtime-backend.md` if backend contract needs documenting

## Read Scope

- crates/codex1-core/src
- crates/codex1/tests
- .codex/skills
- docs

## Write Scope

- crates/codex1-core/src
- crates/codex1/tests
- .codex/skills
- docs
- PLANS/manual-clarify-handoff-boundary/specs/manual_clarify_handoff_runtime

## Interfaces And Contracts Touched

- `init-mission` locked clarify closeout semantics.
- Stop-hook behavior for ratified manual clarify.
- Manual versus autopilot boundary.

## Implementation Shape

Represent manual clarify completion as a durable non-terminal user handoff, likely `needs_user` with `resume_mode = yield_to_user`, `waiting_for = manual_plan_invocation`, and a canonical request to invoke `$plan` manually or `$autopilot` for automatic continuation.

## Proof-Of-Completion Expectations

- Manual ratified clarify Stop-hook emits no block decision and no `Start $plan` blocking reason.
- Manual ratified clarify state contains a durable request for explicit `$plan` invocation.
- Autopilot skill contract still says it continues from clarify to plan.
- Existing runtime and mission validation remain green.

## Non-Breakage Expectations

- Clarify waiting questions still behave as `needs_user`.
- Planning remains available when the user explicitly invokes `$plan`.
- Stop-hook still blocks genuine actionable execution/review/repair/replan states.

## Review Lenses

- spec_conformance
- correctness
- interface_compatibility
- evidence_adequacy

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
- `crates/codex1-core/src/runtime.rs`
- `.codex/skills/autopilot/SKILL.md`

## Freshness Notes

- Current for the implemented manual clarify handoff runtime change.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
