# Spec Notes

- Mission id: `manual-clarify-handoff-boundary`
- Spec id: `manual_clarify_handoff_runtime`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Locked `init-mission` clarify handoff now emits durable `needs_user` /
  `yield_to_user` semantics by default, with `waiting_for =
  manual_plan_invocation`.
- Manual ratified clarify Stop-hook now yields a system message asking the user
  to invoke `$plan` instead of blocking with an auto-plan instruction.
- `$clarify`, `$autopilot`, and `docs/runtime-backend.md` now document that
  manual clarify stops at the handoff while autopilot may consume it and
  continue into planning.
- Added a runtime integration test for manual ratified clarify handoff behavior.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
