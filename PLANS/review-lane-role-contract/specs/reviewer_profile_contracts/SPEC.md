---
artifact: workstream-spec
mission_id: review-lane-role-contract
spec_id: reviewer_profile_contracts
version: 1
spec_revision: 2
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 10
blueprint_fingerprint: sha256:73643e5ae5a1c12e2c80c3b51aafda42fd133e8eb835b7c9d1e19d72be9bd665
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

Define review profiles, model routing, severity handling, six-loop state, and findings-only child output contracts.

## In Scope

- Define local/spec, integration, mission-close, and code bug/correctness review profiles.
- Encode locked model routing.
- Define child reviewer output schema.
- Define loop aggregation semantics.

## Out Of Scope

- Runtime stop-hook enforcement.
- Actual subagent spawning orchestration.
- Changing the six-loop cap or preserving `$review`.

## Dependencies

- `review_loop_skill_surface`.

## Touched Surfaces

- `.codex/skills/review-loop/SKILL.md`
- `.codex/skills/internal-orchestration/SKILL.md`
- `docs/MULTI-AGENT-V2-GUIDE.md`
- `docs/runtime-backend.md`

## Read Scope

- .codex/skills
- docs
- crates/codex1/src/commands/qualify.rs

## Write Scope

- .codex/skills
- docs
- PLANS/review-lane-role-contract/specs/reviewer_profile_contracts

## Interfaces And Contracts Touched

- Reviewer profile contract.
- Child reviewer output schema.
- Review severity semantics.

## Implementation Shape

Add visible profile definitions to `$review-loop` and supporting docs.

## Proof-Of-Completion Expectations

- Profiles document lane name, purpose, model, count policy, input bundle, and output schema.
- P0/P1/P2 and P3 cleanliness semantics are explicit.
- Six-loop cap and replan routing are explicit.

## Non-Breakage Expectations

- Profiles do not introduce `gpt-5.4-mini` as a default blocking review model.

## Review Lenses

- spec_conformance
- correctness
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
- `.codex/config.toml`

## Freshness Notes

- Current for Outcome Lock revision 1 and the implemented reviewer profile
  contract updates.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
