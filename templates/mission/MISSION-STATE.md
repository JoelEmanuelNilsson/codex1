---
artifact: mission-state
mission_id: "{{MISSION_ID}}"
root_mission_id: "{{ROOT_MISSION_ID}}"
parent_mission_id: "{{PARENT_MISSION_ID}}"
version: 1
clarify_status: "clarifying"
slug: "{{MISSION_SLUG}}"
current_lock_revision: null
reopened_from_lock_revision: null
---

# Mission State

## Objective Snapshot

- Mission title: {{MISSION_TITLE}}
- Current interpreted objective: {{INTERPRETED_OBJECTIVE}}
- Current phase hint: {{CURRENT_PHASE_HINT}}

## Ambiguity Register

| Dimension | Score (0-3) | Why it still matters | Planned reducer | Provenance |
| --- | --- | --- | --- | --- |
| Objective clarity | 3 | {{OBJECTIVE_CLARITY_NOTE}} | {{OBJECTIVE_CLARITY_REDUCER}} | {{OBJECTIVE_CLARITY_SOURCE}} |
| Success proof | 3 | {{SUCCESS_PROOF_NOTE}} | {{SUCCESS_PROOF_REDUCER}} | {{SUCCESS_PROOF_SOURCE}} |
| Protected surfaces | 3 | {{PROTECTED_SURFACES_NOTE}} | {{PROTECTED_SURFACES_REDUCER}} | {{PROTECTED_SURFACES_SOURCE}} |
| Tradeoff vetoes | 3 | {{TRADEOFF_NOTE}} | {{TRADEOFF_REDUCER}} | {{TRADEOFF_SOURCE}} |
| Scope boundary | 3 | {{SCOPE_NOTE}} | {{SCOPE_REDUCER}} | {{SCOPE_SOURCE}} |
| Autonomy boundary | 3 | {{AUTONOMY_NOTE}} | {{AUTONOMY_REDUCER}} | {{AUTONOMY_SOURCE}} |
| Baseline facts | 2 | {{BASELINE_FACTS_NOTE}} | {{BASELINE_FACTS_REDUCER}} | {{BASELINE_FACTS_SOURCE}} |
| Rollout or migration constraints | 2 | {{ROLLOUT_NOTE}} | {{ROLLOUT_REDUCER}} | {{ROLLOUT_SOURCE}} |

## Candidate Success Criteria

- {{SUCCESS_CRITERION_1}}
- {{SUCCESS_CRITERION_2}}
- {{SUCCESS_CRITERION_3}}

## Protected Surface Hypotheses

- {{PROTECTED_SURFACE_1}}
- {{PROTECTED_SURFACE_2}}
- {{PROTECTED_SURFACE_3}}

## Baseline Repo Facts

| Fact | Provenance | Evidence ref | Confidence |
| --- | --- | --- | --- |
| {{BASELINE_FACT_1}} | {{BASELINE_FACT_1_SOURCE}} | {{BASELINE_FACT_1_EVIDENCE}} | {{BASELINE_FACT_1_CONFIDENCE}} |
| {{BASELINE_FACT_2}} | {{BASELINE_FACT_2_SOURCE}} | {{BASELINE_FACT_2_EVIDENCE}} | {{BASELINE_FACT_2_CONFIDENCE}} |
| {{BASELINE_FACT_3}} | {{BASELINE_FACT_3_SOURCE}} | {{BASELINE_FACT_3_EVIDENCE}} | {{BASELINE_FACT_3_CONFIDENCE}} |

## Open Assumptions

- {{ASSUMPTION_1}}
- {{ASSUMPTION_2}}
- {{ASSUMPTION_3}}

## Highest-Value Next Question

{{NEXT_QUESTION}}

## Feasibility Notes

- Probe used: {{FEASIBILITY_PROBE}}
- Result: {{FEASIBILITY_RESULT}}
- Constraint surfaced: {{FEASIBILITY_CONSTRAINT}}
