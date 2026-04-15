---
artifact: outcome-lock
mission_id: "{{MISSION_ID}}"
root_mission_id: "{{ROOT_MISSION_ID}}"
parent_mission_id: {{PARENT_MISSION_ID}}
version: 1
lock_revision: 1
status: "draft"
lock_posture: "unconstrained"
slug: "{{MISSION_SLUG}}"
---

# Outcome Lock

## Objective

{{LOCKED_OBJECTIVE}}

## Done-When Criteria

- {{DONE_WHEN_1}}
- {{DONE_WHEN_2}}
- {{DONE_WHEN_3}}

## Success Measures

- {{SUCCESS_MEASURE_1}}
- {{SUCCESS_MEASURE_2}}
- {{SUCCESS_MEASURE_3}}

## Protected Surfaces

- {{PROTECTED_SURFACE_1}}
- {{PROTECTED_SURFACE_2}}
- {{PROTECTED_SURFACE_3}}

## Unacceptable Tradeoffs

- {{UNACCEPTABLE_TRADEOFF_1}}
- {{UNACCEPTABLE_TRADEOFF_2}}
- {{UNACCEPTABLE_TRADEOFF_3}}

## Non-Goals

- {{NON_GOAL_1}}
- {{NON_GOAL_2}}
- {{NON_GOAL_3}}

## Autonomy Boundary

- Codex may decide later without asking: {{AUTONOMOUS_DECISIONS}}
- Codex must ask before deciding: {{USER_ONLY_DECISIONS}}

## Locked Field Discipline

The fields above for objective, done-when criteria, protected surfaces,
unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are
locked fields. Change them only through an explicit reopen or superseding lock
revision, never by silent mutation.

Baseline facts and rollout or migration constraints are also revision-gated:
extend them only through an explicit lock revision when new truth materially
changes the destination contract.

## Baseline Current Facts

- {{BASELINE_FACT_1}}
- {{BASELINE_FACT_2}}
- {{BASELINE_FACT_3}}

## Rollout Or Migration Constraints

- {{ROLLOUT_CONSTRAINT_1}}
- {{ROLLOUT_CONSTRAINT_2}}
- {{ROLLOUT_CONSTRAINT_3}}

## Remaining Low-Impact Assumptions

- {{LOW_IMPACT_ASSUMPTION_1}}
- {{LOW_IMPACT_ASSUMPTION_2}}
- {{LOW_IMPACT_ASSUMPTION_3}}

## Feasibility Constraints

Use this section only when `lock_posture = constrained`.

- {{FEASIBILITY_CONSTRAINT_1}}
- {{FEASIBILITY_CONSTRAINT_2}}

## Reopen Conditions

- {{REOPEN_CONDITION_1}}
- {{REOPEN_CONDITION_2}}
- {{REOPEN_CONDITION_3}}

## Provenance

### User-Stated Intent

- {{USER_INTENT_1}}
- {{USER_INTENT_2}}

### Repo-Grounded Facts

- {{REPO_FACT_1}}
- {{REPO_FACT_2}}

### Codex Clarifying Synthesis

- {{SYNTHESIS_1}}
- {{SYNTHESIS_2}}
