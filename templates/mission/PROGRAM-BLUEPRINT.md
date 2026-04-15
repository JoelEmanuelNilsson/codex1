---
artifact: program-blueprint
mission_id: "{{MISSION_ID}}"
version: 1
lock_revision: 1
blueprint_revision: 1
plan_level: {{PLAN_LEVEL}}
problem_size: {{PROBLEM_SIZE}}
status: "draft"
---

# Program Blueprint

## 1. Locked Mission Reference

- Mission id: `{{MISSION_ID}}`
- Lock revision: `{{LOCK_REVISION}}`
- Lock fingerprint: `{{LOCK_FINGERPRINT}}`
- Outcome summary: {{LOCK_SUMMARY}}

## 2. Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | {{TRUTH_ROW_1}} | {{TRUTH_ROW_1_EVIDENCE}} | {{TRUTH_ROW_1_SOURCE_TYPE}} | {{TRUTH_ROW_1_OBSERVATION_BASIS}} | {{TRUTH_ROW_1_OBSERVED_STATE}} | {{TRUTH_ROW_1_FRESHNESS}} | {{TRUTH_ROW_1_CONFIDENCE}} |
| T2 | assumption | {{TRUTH_ROW_2}} | {{TRUTH_ROW_2_EVIDENCE}} | {{TRUTH_ROW_2_SOURCE_TYPE}} | {{TRUTH_ROW_2_OBSERVATION_BASIS}} | {{TRUTH_ROW_2_OBSERVED_STATE}} | {{TRUTH_ROW_2_FRESHNESS}} | {{TRUTH_ROW_2_CONFIDENCE}} |
| T3 | unknown | {{TRUTH_ROW_3}} | {{TRUTH_ROW_3_EVIDENCE}} | {{TRUTH_ROW_3_SOURCE_TYPE}} | {{TRUTH_ROW_3_OBSERVATION_BASIS}} | {{TRUTH_ROW_3_OBSERVED_STATE}} | {{TRUTH_ROW_3_FRESHNESS}} | {{TRUTH_ROW_3_CONFIDENCE}} |

## 3. System Model

- Touched surfaces: {{TOUCHED_SURFACES}}
- Boundary summary: {{BOUNDARY_SUMMARY}}
- Hidden coupling summary: {{COUPLING_SUMMARY}}

## 4. Invariants And Protected Behaviors

- {{INVARIANT_1}}
- {{INVARIANT_2}}
- {{INVARIANT_3}}

## 5. Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | {{PROOF_ROW_1}} | {{PROOF_ROW_1_EVIDENCE}} | {{PROOF_ROW_1_OWNER}} | yes |
| P2 | {{PROOF_ROW_2}} | {{PROOF_ROW_2_EVIDENCE}} | {{PROOF_ROW_2_OWNER}} | yes |
| P3 | {{PROOF_ROW_3}} | {{PROOF_ROW_3_EVIDENCE}} | {{PROOF_ROW_3_OWNER}} | {{PROOF_ROW_3_BLOCKING}} |

## 6. Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | {{DECISION_1_QUESTION}} | {{DECISION_1_WHY}} | critical | {{DECISION_1_STATUS}} | {{DECISION_1_EVIDENCE}} |
| DO-2 | {{DECISION_2_QUESTION}} | {{DECISION_2_WHY}} | major | {{DECISION_2_STATUS}} | {{DECISION_2_EVIDENCE}} |

## 7. In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| {{WORK_ITEM_1}} | runnable_frontier | {{WORK_ITEM_1_REASON}} | {{WORK_ITEM_1_OWNER}} | yes |
| {{WORK_ITEM_2}} | near_frontier | {{WORK_ITEM_2_REASON}} | {{WORK_ITEM_2_OWNER}} | {{WORK_ITEM_2_FINISH}} |
| {{WORK_ITEM_3}} | proof_gated_spike | {{WORK_ITEM_3_REASON}} | {{WORK_ITEM_3_OWNER}} | {{WORK_ITEM_3_FINISH}} |
| {{WORK_ITEM_4}} | provisional_backlog | {{WORK_ITEM_4_REASON}} | {{WORK_ITEM_4_OWNER}} | no |
| {{WORK_ITEM_5}} | deferred_or_descoped | {{WORK_ITEM_5_REASON}} | {{WORK_ITEM_5_OWNER}} | no |

## 8. Option Set

If only one viable route survives, replace the alternatives below with an
explicit note that no second viable route survived and why.

- Option A: {{OPTION_A}}
- Option B: {{OPTION_B}}

## 9. Selected Architecture

{{SELECTED_ARCHITECTURE}}

## 10. Rejected Alternatives And Rationale

If only one viable route survives, keep a short invalidation rationale here
instead of deleting the section silently.

- {{REJECTED_ALTERNATIVE_1}}
- {{REJECTED_ALTERNATIVE_2}}

## 11. Migration / Rollout / Rollback Posture

Delete this section when rollout sensitivity is genuinely absent.

- Migration posture: {{MIGRATION_POSTURE}}
- Rollout posture: {{ROLLOUT_POSTURE}}
- Rollback posture: {{ROLLBACK_POSTURE}}

## 12. Review Bundle Design

- Mandatory review lenses: {{MANDATORY_REVIEW_LENSES}}
- Required receipts: {{REQUIRED_RECEIPTS}}
- Required changed-file context: {{REQUIRED_CHANGED_FILE_CONTEXT}}
- Mission-close claims requiring integrated judgment: {{MISSION_CLOSE_CLAIMS}}

## 13. Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| {{SPEC_ID_1}} | {{SPEC_ID_1_PURPOSE}} | {{SPEC_ID_1_STATUS}} | {{SPEC_ID_1_OWNER}} | {{SPEC_ID_1_DEPENDS_ON}} |
| {{SPEC_ID_2}} | {{SPEC_ID_2_PURPOSE}} | {{SPEC_ID_2_STATUS}} | {{SPEC_ID_2_OWNER}} | {{SPEC_ID_2_DEPENDS_ON}} |

## 14. Execution Graph And Safe-Wave Rules

Delete this section when there is only one real runnable node.

- Graph summary: {{GRAPH_SUMMARY}}
- Safe-wave rule 1: {{SAFE_WAVE_RULE_1}}
- Safe-wave rule 2: {{SAFE_WAVE_RULE_2}}

## 15. Risks And Unknowns

- {{RISK_1}}
- {{RISK_2}}
- {{RISK_3}}

## 16. Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| {{DECISION_LOG_ID_1}} | {{DECISION_LOG_STATEMENT_1}} | {{DECISION_LOG_RATIONALE_1}} | {{DECISION_LOG_EVIDENCE_1}} | {{DECISION_LOG_AFFECTED_ARTIFACTS_1}} | {{DECISION_LOG_REVISION_1}} |

## 17. Replan Policy

- Reopen Outcome Lock when: {{LOCK_REOPEN_RULE}}
- Reopen blueprint when: {{BLUEPRINT_REOPEN_RULE}}
- Reopen execution package when: {{PACKAGE_REOPEN_RULE}}
- Local repair allowed when: {{LOCAL_REPAIR_RULE}}
