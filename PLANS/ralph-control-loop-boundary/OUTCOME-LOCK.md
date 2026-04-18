---
artifact: outcome-lock
mission_id: "ralph-control-loop-boundary"
root_mission_id: "ralph-control-loop-boundary"
parent_mission_id: null
version: 1
lock_revision: 1
status: "locked"
lock_posture: "unconstrained"
slug: "ralph-control-loop-boundary"
---

# Outcome Lock

## Objective

Codex1 must make Ralph continuation scoped instead of global: Ralph may enforce continuation only for the main parent/orchestrator thread while an explicit loop workflow is active, and must not block normal user communication or any subagent from stopping.

## Done-When Criteria

- Normal user conversation, clarification, discussion, interrupts, and manual skill invocations that are not active loop continuations can stop without being blocked by open mission gates.
- All subagents are Ralph-exempt by default; they can stop normally, and the parent thread remains responsible for handling missing, partial, invalid, or useful subagent output.
- `$plan`, `$execute`, `$review-loop`, and `$autopilot` have an explicit parent-owned continuation mode/lease so Ralph still enforces the intended loop when the user actually entered a loop workflow.
- A user-facing pause/close escape exists, or an equivalent durable control-plane state exists, so the user can intentionally suspend Ralph continuation without editing hook files by hand.
- Qualification or runtime tests prove user-interaction turns and subagent turns are not blocked by parent mission gates, while explicit parent loop turns still are.

## Success Measures

- A stop-hook probe for a normal user-interaction turn with an open/failed gate yields instead of blocking.
- A stop-hook probe for a generic subagent turn with an open/failed gate yields instead of blocking.
- A stop-hook probe for an active parent `$execute`/`$review-loop` lease with an open/failed gate still blocks with the correct next action.
- Existing waiting, selection, review-gate, and manual-clarify handoff behavior remains correct.
- Review agents no longer need bespoke `NONE`/findings completion logic to escape Ralph; their outputs are judged by the parent, not by the stop hook.

## Protected Surfaces

- Parent-owned mission truth under `PLANS/<mission-id>/` and `.ralph/missions/<mission-id>/`.
- Stop-hook behavior in `codex1 internal stop-hook` and `codex1-core::resolve_stop_hook_output`.
- Public skill semantics for `clarify`, `plan`, `execute`, `review-loop`, and `autopilot`.
- Review-loop delegated authority: subagents must not own gates, closeouts, ledgers, packages, specs, or mission completion.
- Manual clarify handoff: `$clarify` does not auto-enter planning unless `$autopilot` owns the workflow.

## Unacceptable Tradeoffs

- Do not make Ralph a global blocker for every assistant stop just because some mission has unfinished work.
- Do not require subagents to prove a special output shape to stop.
- Do not weaken Ralph enforcement for explicit parent loop workflows.
- Do not rely only on prompt wording to distinguish parent loops, subagents, and user-interaction turns.
- Do not require the user to move or edit `.codex/hooks.json` as the normal escape hatch.

## Non-Goals

- Do not finish the broader review-loop-delegated-review-only mission in this lock.
- Do not redesign all mission planning, execution, or review contracts beyond the control-loop boundary needed here.
- Do not build an external babysitter/runtime outside native Codex hooks and visible mission artifacts.

## Autonomy Boundary

- Codex may decide later without asking: exact internal data shape for the continuation lease/session-control artifact, exact command names for pause/close if they preserve the locked UX, test decomposition, and whether the implementation stores mode in `.ralph/session-control.json` or an equivalent machine-readable surface.
- Codex must ask before deciding: removing existing public skills, changing the product claim that `$autopilot` can run end-to-end, or making subagents mission-truth owners.

## Locked Field Discipline

The fields above for objective, done-when criteria, protected surfaces,
unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are
locked fields. Change them only through an explicit reopen or superseding lock
revision, never by silent mutation.

Baseline facts and rollout or migration constraints are also revision-gated:
extend them only through an explicit lock revision when new truth materially
changes the destination contract.

## Baseline Current Facts

- The current project Stop hook delegates to `codex1 internal stop-hook` from `.codex/hooks.json`.
- The current stop-hook path bypasses only narrow findings-only reviewer-lane metadata before calling `resolve_stop_hook_output`.
- `resolve_stop_hook_output` currently calls `resolve_resume`, so ordinary parent conversation can inherit open or failed mission-gate blockers.
- The user manually moved/disabled the hook as an emergency escape, proving the product lacks a first-class pause/close boundary.
- The active reviewer-output-inbox review showed that reviewer subagents can still be pushed into parent-style continuation when Ralph treats them as normal sessions.

## Rollout Or Migration Constraints

- Preserve support for existing Ralph mission artifacts while adding the new control-plane boundary.
- Keep the native Codex hooks-first design; do not add an external controller.
- Existing repos with a hook installed should get a clear setup/restore path for the new behavior.

## Remaining Low-Impact Assumptions

- The exact pause/close skill name can be chosen during planning.
- The exact session-control file name can be chosen during planning.
- The first implementation can focus on local repo/native hook behavior before broader multi-repo polish.

## Feasibility Constraints

Use this section only when `lock_posture = constrained`.

- None.

## Reopen Conditions

- Reopen the lock if native Codex cannot expose enough stop-hook input to distinguish user-interaction turns from parent loop turns.
- Reopen the lock if subagents cannot be reliably identified or globally exempted without weakening parent loop enforcement.
- Reopen the lock if the user changes the desired behavior from parent-loop-only enforcement to broader always-on mission enforcement.

## Provenance

### User-Stated Intent

- Ralph loop should apply to main orchestrating Codex during `$plan`, `$execute`, `$review-loop`, and `$autopilot`, not to everything.
- Subagents should escape Ralph by default; the parent decides what to do with their outputs.
- Human communication must override/interrupt Ralph blocking so the user can discuss, clarify, redirect, or pause.
- The current manual workaround of moving the hook is acceptable only as an emergency escape, not product UX.

### Repo-Grounded Facts

- `crates/codex1/src/internal/mod.rs` has a narrow `stop_hook_input_is_findings_only_review_lane` bypass.
- `crates/codex1-core/src/runtime.rs` routes normal stop-hook handling through `resolve_resume`.
- `PLANS/review-loop-delegated-review-only` currently contains an unresolved failed review branch, which is enough to demonstrate the unwanted global blocking behavior.

### Codex Clarifying Synthesis

- The product needs a scoped continuation lease/control-plane model, not more prompt instructions.
- Open mission gates should block only active autonomous parent leases, never ordinary conversation or subagent completion.
