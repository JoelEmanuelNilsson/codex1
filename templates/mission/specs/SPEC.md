---
artifact: workstream-spec
mission_id: "{{MISSION_ID}}"
spec_id: "{{SPEC_ID}}"
version: 1
spec_revision: 1
artifact_status: "draft"
packetization_status: "near_frontier"
execution_status: "not_started"
owner_mode: "solo"
blueprint_revision: 1
blueprint_fingerprint: "{{BLUEPRINT_FINGERPRINT}}"
spec_fingerprint: "{{SPEC_FINGERPRINT}}"
replan_boundary:
  local_repair_allowed: false
  trigger_matrix:
    - trigger_code: write_scope_expansion
      reopen_layer: "{{WRITE_SCOPE_REOPEN_LAYER}}"
    - trigger_code: interface_contract_change
      reopen_layer: "{{INTERFACE_REOPEN_LAYER}}"
    - trigger_code: dependency_truth_change
      reopen_layer: "{{DEPENDENCY_REOPEN_LAYER}}"
    - trigger_code: proof_obligation_change
      reopen_layer: "{{PROOF_REOPEN_LAYER}}"
    - trigger_code: review_contract_change
      reopen_layer: "{{REVIEW_REOPEN_LAYER}}"
    - trigger_code: protected_surface_change
      reopen_layer: "{{PROTECTED_SURFACE_REOPEN_LAYER}}"
    - trigger_code: migration_rollout_change
      reopen_layer: "{{ROLLOUT_REOPEN_LAYER}}"
    - trigger_code: outcome_lock_change
      reopen_layer: "{{OUTCOME_REOPEN_LAYER}}"
---

# Workstream Spec

## Purpose

{{SPEC_PURPOSE}}

## In Scope

- {{IN_SCOPE_1}}
- {{IN_SCOPE_2}}
- {{IN_SCOPE_3}}

## Out Of Scope

- {{OUT_OF_SCOPE_1}}
- {{OUT_OF_SCOPE_2}}
- {{OUT_OF_SCOPE_3}}

## Dependencies

- {{DEPENDENCY_1}}
- {{DEPENDENCY_2}}
- {{DEPENDENCY_3}}

## Touched Surfaces

- {{TOUCHED_SURFACE_1}}
- {{TOUCHED_SURFACE_2}}
- {{TOUCHED_SURFACE_3}}

## Read Scope

- {{READ_PATH_1}}
- {{READ_PATH_2}}
- {{READ_PATH_3}}

## Write Scope

- {{WRITE_PATH_1}}
- {{WRITE_PATH_2}}
- {{WRITE_PATH_3}}

## Interfaces And Contracts Touched

- {{INTERFACE_1}}
- {{INTERFACE_2}}
- {{INTERFACE_3}}

## Implementation Shape

{{IMPLEMENTATION_SHAPE}}

## Proof-Of-Completion Expectations

- {{PROOF_EXPECTATION_1}}
- {{PROOF_EXPECTATION_2}}
- {{PROOF_EXPECTATION_3}}

## Non-Breakage Expectations

- {{NON_BREAKAGE_EXPECTATION_1}}
- {{NON_BREAKAGE_EXPECTATION_2}}
- {{NON_BREAKAGE_EXPECTATION_3}}

## Review Lenses

- {{REVIEW_LENS_1}}
- {{REVIEW_LENS_2}}
- {{REVIEW_LENS_3}}

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| write_scope_expansion | {{WRITE_SCOPE_REOPEN_LAYER}} |
| interface_contract_change | {{INTERFACE_REOPEN_LAYER}} |
| dependency_truth_change | {{DEPENDENCY_REOPEN_LAYER}} |
| proof_obligation_change | {{PROOF_REOPEN_LAYER}} |
| review_contract_change | {{REVIEW_REOPEN_LAYER}} |
| protected_surface_change | {{PROTECTED_SURFACE_REOPEN_LAYER}} |
| migration_rollout_change | {{ROLLOUT_REOPEN_LAYER}} |
| outcome_lock_change | {{OUTCOME_REOPEN_LAYER}} |

## Truth Basis Refs

- {{TRUTH_BASIS_REF_1}}
- {{TRUTH_BASIS_REF_2}}
- {{TRUTH_BASIS_REF_3}}

## Freshness Notes

- {{FRESHNESS_NOTE_1}}
- {{FRESHNESS_NOTE_2}}

## Support Files

- `REVIEW.md` records per-spec review context and dispositions.
- `NOTES.md` holds non-authoritative local notes or spike results.
- `RECEIPTS/` stores proof receipts that support completion claims.
