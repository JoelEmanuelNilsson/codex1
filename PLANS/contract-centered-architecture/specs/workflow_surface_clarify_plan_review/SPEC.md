---
artifact: workstream-spec
mission_id: contract-centered-architecture
spec_id: workflow_surface_clarify_plan_review
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

Strengthen the public clarify, plan, and review skill surfaces against the new contracts.

## In Scope

- Upgrade `clarify` to ask higher-leverage, lower-effort questions until destination truth is genuinely ready to lock.
- Upgrade `plan` to encode the full planning program, critique loop, packetization expectations, and bounded orchestration posture from the blueprinted route.
- Upgrade `review` to stay bundle-bound, evidence-bound, and mission-close honest against the strengthened proof contracts.
- Keep the public workflow semantics visible in skills rather than hiding them in private orchestration code.

## Out Of Scope

- Full execution or autopilot authority behavior, which is covered by a separate workstream.
- Rebuilding skills into a wrapper-runtime product.
- Thin cosmetic prompt edits without contract-level improvement.

## Dependencies

- The mission-contract kernel, artifact registry, and qualification pipeline should already define stronger underlying truth and proof surfaces.
- Planning blueprint revision 1 remains the governing route truth.

## Touched Surfaces

- `.codex/skills/clarify/SKILL.md`
- `.codex/skills/plan/SKILL.md`
- `.codex/skills/review/SKILL.md`
- supporting docs or runtime-backend notes needed to keep the public workflow explicit

## Read Scope

- .codex/skills
- docs/codex1-prd.md
- docs/runtime-backend.md
- plans/contract-centered-architecture/specs/workflow_surface_clarify_plan_review

## Write Scope

- .codex/skills
- docs/runtime-backend.md
- plans/contract-centered-architecture/specs/workflow_surface_clarify_plan_review

## Interfaces And Contracts Touched

- Public skill UX contract.
- Clarify lock-readiness contract.
- Planning-program visibility contract.
- Review bundle and mission-close contract.

## Implementation Shape

Keep skills as first-class explicit workflow contracts. Borrow OMX's strongest ideas about interview pressure and critique loops, but express them in Codex1's native skills-first, artifact-centered product model.

## Proof-Of-Completion Expectations

- clarify questions materially reduce ambiguity with less user effort and stronger destination truth
- plan skill instructions match the required planning program and package gate expectations
- review skill instructions preserve independent, bundle-bound, mission-close-honest review behavior
- changed skills remain aligned with runtime-backend and PRD contracts

## Non-Breakage Expectations

- existing public skill names remain intact
- skill changes do not become a hidden workflow engine by themselves
- clarify, plan, and review remain legible to a fresh Codex session

## Review Lenses

- spec_conformance
- correctness
- evidence_adequacy

## Replan Boundary

- Reopen planning if stronger public-skill UX requires a materially different product contract than the locked mission assumes.
- Reopen planning if skill improvements would only be possible by reintroducing hidden wrapper-runtime ownership.

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- /Users/joel/oh-my-codex/skills/deep-interview/SKILL.md
- /Users/joel/oh-my-codex/skills/ralplan/SKILL.md

## Freshness Notes

- Current for lock revision 1 and the present Codex1 versus OMX workflow comparison.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
