---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: execute_autopilot_governance
version: 1
spec_revision: 4
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

Make execute and autopilot fully autonomous, parity-safe, and Ralph-honest.

## In Scope

- Encode the bounded-planning versus fully autonomous-execution contract into the execute or autopilot route and proof story.
- Make `autopilot` a thin composition over the same clarify, plan, execute, and review branches rather than a second workflow engine.
- Strengthen execute or autopilot proof and qualification hooks for no-false-terminal, package honesty, review-gate honesty, contradiction handling, and native child-lane reconciliation.
- Preserve bounded internal orchestration as support work rather than child-owned mission truth.

## Out Of Scope

- Replacing native Codex orchestration with tmux or team runtime ownership.
- Public-skill clarify, plan, or review improvements outside the execution or autopilot branch contract.
- Support-surface helper mutation redesign except where qualification needs it.

## Dependencies

- The mission-contract kernel and qualification evidence pipeline should already be in place.
- Planning blueprint revision 1 remains the governing route truth.

## Touched Surfaces

- `.codex/skills/execute/SKILL.md`
- `.codex/skills/autopilot/SKILL.md`
- Ralph runtime and resume logic in `codex1-core`
- qualification and orchestration docs that prove the route honestly

## Read Scope

- .codex/skills
- crates/codex1-core/src
- crates/codex1/src/commands
- docs/qualification
- docs/MULTI-AGENT-V2-GUIDE.md
- plans/contract-centered-architecture/specs/execute_autopilot_governance

## Write Scope

- .codex/skills
- crates/codex1-core/src
- crates/codex1/src/commands
- docs/qualification
- docs/MULTI-AGENT-V2-GUIDE.md
- plans/contract-centered-architecture/specs/execute_autopilot_governance

## Interfaces And Contracts Touched

- Execute package-entry and review-routing contract.
- Autopilot parity contract.
- Ralph continuation and no-false-terminal contract.
- Native multi-agent reconciliation contract.

## Implementation Shape

Keep execute and autopilot as explicit skill-level branches over the same artifact and package truth. Strengthen the core runtime and proof surfaces they depend on, but do not move product semantics into hidden wrappers.

## Proof-Of-Completion Expectations

- execute never starts without a passed package and remains honest about review, repair, and replan branches
- autopilot produces the same artifact truth and gate outcomes as the manual path
- no-false-terminal, child-lane reconciliation, and waiting-identity proofs remain green after the touched changes
- the fully autonomous execution promise is backed by explicit runtime and qualification evidence

## Non-Breakage Expectations

- native multi-agent depth and authority rules remain intact
- skills remain the public surface
- manual execution remains possible and parity-safe

## Review Lenses

- correctness
- safety_security_policy
- operability_rollback_observability
- evidence_adequacy

## Replan Boundary

- Reopen planning if fully autonomous execution cannot be proven honestly on the supported native Codex surface.
- Reopen planning if preserving autopilot or manual parity requires a materially different execution model.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- docs/qualification/README.md
- docs/MULTI-AGENT-V2-GUIDE.md

## Freshness Notes

- Current for lock revision 1 and the present autonomous-execution product promise.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
