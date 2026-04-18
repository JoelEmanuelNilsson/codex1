---
artifact: outcome-lock
mission_id: manual-clarify-handoff-boundary
root_mission_id: manual-clarify-handoff-boundary
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: manual-clarify-handoff-boundary
---
# Outcome Lock

## Objective

Fix Codex1's manual clarify handoff so a ratified `$clarify` mission does not cause the Ralph Stop hook to block and push `$plan`. Manual `$clarify` should stop at a clean handoff and wait for the user to explicitly invoke `$plan`; `$autopilot` is the workflow that continues automatically from clarify into planning.

## Done-When Criteria

- A manually invoked `$clarify` that ratifies `OUTCOME-LOCK.md` leaves a durable non-terminal user handoff instead of a blocking Stop-hook instruction to start `$plan`.
- `$autopilot` still continues from ratified clarify into `$plan` when it owns the end-to-end workflow.
- Stop-hook/resume behavior distinguishes manual handoff from autopilot continuation with tests or an equivalent machine-checkable proof.

## Success Measures

- Manual clarify stop output does not block with `Start $plan ...` after lock ratification.
- Autopilot can still advance through clarify -> plan when invoked as autopilot.
- Durable mission artifacts make the handoff explicit and do not rely on hidden chat state.

## Protected Surfaces

- `$clarify`, `$plan`, `$execute`, `$autopilot`, and `$review-loop` skill semantics.
- Ralph Stop-hook and resume behavior.
- Mission closeout/state semantics for `needs_user`, `continue_required`, `next_phase`, and `resume_mode`.

## Unacceptable Tradeoffs

- Do not make manual `$clarify` automatically start planning.
- Do not break `$autopilot` continuation.
- Do not solve this with wrapper-runtime behavior or hidden chat memory.

## Non-Goals

- Redesign planning quality.
- Change the locked mission destination flow beyond clarify-to-plan handoff semantics.
- Remove user ability to manually call `$plan` after clarify.

## Autonomy Boundary

- Codex may decide later without asking: exact runtime representation, such as `needs_user` waiting handoff versus another non-blocking state, if it preserves manual stop and autopilot continuation.
- Codex must ask before deciding: making manual `$clarify` auto-plan, making autopilot stop after clarify, or changing the public skill names.

## Locked Field Discipline

The fields above are locked. Change only through explicit reopen.

## Baseline Current Facts

- Current `init-mission` locked clarify path emits `continue_required`, `resume_mode = continue`, `next_phase = planning`, and a planning next action.
- Current Stop-hook blocks on actionable non-terminal resume states, which caused the screenshot behavior.
- `$autopilot` is documented as the branch router that keeps looping across clarify, plan, execute, review-loop, and closure.

## Rollout Or Migration Constraints

- Preserve manual-path parity except for the intended difference that manual mode yields at skill boundaries while autopilot consumes those boundaries.

## Remaining Low-Impact Assumptions

- The implementation may use durable `needs_user` waiting with canonical request to invoke `$plan`, unless planning finds a cleaner equivalent.
- The exact marker distinguishing autopilot-owned continuation from manual clarify handoff is delegated to planning.

## Feasibility Constraints

- None identified.

## Reopen Conditions

- Reopen if native Codex cannot distinguish manual skill invocation from autopilot-owned flow without changing public workflow semantics.
- Reopen if `needs_user` handoff proves incompatible with resume selection or mission state validation.

## Provenance

### User-Stated Intent

- The screenshot showed a bug: Ralph blocked after clarify and pushed `$plan` even though the user had not invoked `$autopilot`.
- Manual flow should be user calls `$clarify`, then manually calls `$plan`, then manually calls `$execute`.
- `$autopilot` should be the flow that continues automatically from clarify to planning.

### Repo-Grounded Facts

- `runtime.rs` currently emits planning as the next action when lock status is locked.
- `stop_output_from_resume_report` blocks actionable non-terminal states.
- `$autopilot` explicitly says it keeps looping while the next branch is machine-clear.

### Codex Clarifying Synthesis

- The product needs an explicit boundary between manual skill handoff and autopilot branch consumption.
