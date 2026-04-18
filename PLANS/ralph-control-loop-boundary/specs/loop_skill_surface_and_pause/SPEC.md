---
artifact: workstream-spec
mission_id: ralph-control-loop-boundary
spec_id: loop_skill_surface_and_pause
version: 1
spec_revision: 5
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

Update the public skill surface so loop leases are acquired only by explicit parent workflows and users have a first-class pause/close escape instead of moving .codex/hooks.json manually.

## In Scope

- Update $plan, $execute, $review-loop, and $autopilot instructions to acquire/refresh the appropriate parent loop lease.
- Update $clarify to preserve the manual handoff unless autopilot owns the workflow.
- Add or define a $close/pause skill surface, or an equivalent public escape command, that pauses/clears the active lease without uninstalling hooks.
- Update internal orchestration docs to state all subagents are Ralph-exempt and parent-owned integration handles missing outputs.
- Add skill/doc tests or validation assertions for the public control-loop contract.

## Out Of Scope

- Changing the locked behavior that explicit parent loops remain Ralph-governed.
- Moving mission truth ownership to subagents.
- Implementing the runtime lease schema if it was not completed by ralph_loop_lease_runtime.

## Dependencies

- ralph_loop_lease_runtime complete and reviewed, including final command names and control-state shape.

## Touched Surfaces

- .codex/skills/plan/SKILL.md
- .codex/skills/execute/SKILL.md
- .codex/skills/review-loop/SKILL.md
- .codex/skills/autopilot/SKILL.md
- .codex/skills/clarify/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- optional .codex/skills/close/SKILL.md or equivalent
- docs/runtime-backend.md
- crates/codex1/tests/runtime_internal.rs

## Read Scope

- .codex/skills
- docs/runtime-backend.md
- crates/codex1/tests/runtime_internal.rs
- PLANS/ralph-control-loop-boundary

## Write Scope

- .codex/skills/plan/SKILL.md
- .codex/skills/execute/SKILL.md
- .codex/skills/review-loop/SKILL.md
- .codex/skills/autopilot/SKILL.md
- .codex/skills/clarify/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- .codex/skills/close/SKILL.md
- docs/runtime-backend.md
- crates/codex1/tests/runtime_internal.rs
- PLANS/ralph-control-loop-boundary/specs/loop_skill_surface_and_pause

## Interfaces And Contracts Touched

- Public loop skill entry/exit contract.
- User pause/close UX.
- Subagent orchestration contract.

## Implementation Shape

Once runtime commands exist, make public skills explicit: loop workflows begin a parent lease, keep it active only for the intended autonomous loop, and close or pause it on user escape. Add a public close/pause skill if that is the clearest surface. Make internal orchestration state that subagents never acquire Ralph leases and should never be blocked by parent mission gates.

## Proof-Of-Completion Expectations

- Skill-surface test asserts loop skills mention lease acquisition and close/pause semantics.
- Runtime doc mentions the public pause/close path.
- cargo test -p codex1 --test runtime_internal ralph_control_loop_boundary --quiet
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id ralph-control-loop-boundary

## Non-Breakage Expectations

- Existing public skills remain present except the intentionally removed $review legacy surface; $review-loop is the public parent/orchestrator review-loop workflow.
- $autopilot still owns clarify -> plan -> execute continuation.
- Manual $clarify still yields to user before $plan.

## Review Lenses

- spec_conformance
- intent_alignment
- interface_compatibility
- evidence_adequacy

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| runtime_command_shape_changed | execution_package |
| user_escape_semantics_changed | blueprint |
| public_skill_removed_or_renamed | mission_lock |
| proof_obligation_change | blueprint |

## Truth Basis Refs

- PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md
- .codex/skills/plan/SKILL.md
- .codex/skills/execute/SKILL.md
- .codex/skills/review-loop/SKILL.md
- .codex/skills/autopilot/SKILL.md


## Freshness Notes

- Refresh this spec if native Codex Stop-hook payloads add or remove reliable parent/subagent/user-turn metadata.
- Refresh this spec if the public skill loop entry model changes before execution.

## Support Files

- REVIEW.md records delegated review outcomes for this spec.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output and command transcripts.
