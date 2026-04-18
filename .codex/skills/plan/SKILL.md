---
name: plan
description: Deep planning workflow for Codex1. Use when the user invokes $plan, asks for a rigorous mission plan, or needs a locked mission turned into a blueprint, frontier specs, and an execution-ready next target.
---

# Plan

Use this public skill to turn a locked mission into a planning package strong
enough that execution does not need to invent architecture, proof, or review
contracts.

## Owns

- `PLANS/<mission-id>/PROGRAM-BLUEPRINT.md`
- `PLANS/<mission-id>/specs/<workstream-id>/SPEC.md`
- optional `PLANS/<mission-id>/blueprint/*` support files when the mission is
  too large for one compact blueprint
- the visible planning handoff recorded in the mission `README.md`

Bootstrap or refresh planning artifacts from:

- `templates/mission/PROGRAM-BLUEPRINT.md`
- `templates/mission/blueprint/README.md`
- `templates/mission/specs/SPEC.md`
- `templates/mission/specs/REVIEW.md`
- `templates/mission/specs/NOTES.md`
- `templates/mission/specs/RECEIPTS/README.md`

Deterministic backend:

- begin a parent planning loop with `codex1 internal begin-loop-lease` using
  `mode = "planning_loop"` before relying on Ralph continuation
- use `codex1 internal materialize-plan` for visible blueprint/spec writeback
- use `codex1 internal compile-execution-package` and
  `codex1 internal validate-execution-package` for the next selected target
- see `docs/runtime-backend.md` for payload shape and state ownership

## Ralph Lease

`$plan` is a parent/orchestrator loop. When the user invokes `$plan`, acquire or
refresh a parent lease before autonomous planning continuation:

```json
{
  "mission_id": "<mission-id>",
  "mode": "planning_loop",
  "owner": "parent-plan",
  "reason": "User invoked $plan."
}
```

Use `codex1 internal begin-loop-lease` for that payload. Clear or pause the
lease only through the explicit close/pause surface or when the workflow leaves
a durable `needs_user`, `hard_blocked`, or terminal reviewed state.

## Planning Posture

- Optimize for route quality, not document minimalism.
- Spend compute freely when ambiguity, risk, or coupling justify it.
- Use bounded subagents when they materially improve critique, research,
  decomposition, or package quality.
- Keep the public planning truth explicit in artifacts; do not hide core route
  choice in orchestration-only reasoning.

## Planning Program

1. Validate the current Outcome Lock before planning further.
2. Compute the effective planning rigor as `max(user_floor, risk_floor)`.
3. Run the mandatory planning methods whose triggers apply:
   - truth register
   - system map
   - boundary and coupling map when needed
   - invariant register
   - proof matrix
   - decision obligations
   - option generation only where real decision obligations exist
   - option research and adversarial critique when more than one viable route
     remains
   - deepening on weak or contradictory areas
   - proof and review design
   - blueprint assembly
   - workstream packetization
   - execution graph and wave design when sequencing is non-trivial
   - execution package gate for the next selected target
4. Run real route pressure:
   keep at least one steelman alternative alive until the selected route has
   survived critique honestly, or record why no other route remains viable.
5. Keep the selected route, proof design, review contract, and packetization in
   sync after route selection. Do not carry stale pre-selection drafts forward.
6. Make every runnable workstream execution-grade:
   bounded scope, explicit proof story, explicit review story, explicit
   replan boundary.
7. Use `internal-orchestration` when bounded subagents materially help, and use
   `internal-replan` if planning proves that a higher layer must reopen.

## Planning Rules

- Be repo-grounded, evidence-driven, and critique-driven.
- Do not invent quota-shaped options when only one viable route survives.
- Do not let execution inherit unresolved architecture questions.
- No work item is runnable unless it has a proof story and review story.
- Compact plans are allowed for bounded low-risk work, but compact does not mean
  shallow.
- The blueprint must make clear:
  selected route, rejected alternatives or invalidation rationale, proof
  matrix, decision obligations, execution graph when needed, and the exact next
  packaged target.

## Completion Bar

Planning is complete only when all of the following are true:

- critical truth is explicit enough to support the chosen route
- critical decision obligations are resolved, escalated, or converted into
  proof-gated spikes
- the chosen route has survived critique strongly enough that execution will not
  invent major missing decisions
- proof and review design are explicit
- the frontier is packetized into bounded specs
- at least the next selected target has a passed execution package gate
- the selected route is strong enough that execution will not need to invent
  major architecture or proof structure on the fly

## Must Not

- stop because one document draft exists
- hide core planning method in private code or vibes
- silently discard earlier valid evidence during replanning
- claim planning completion while the next execution target is still unpackaged
- package a frontier slice whose dependencies or review contract are still
  fuzzy

## Return Shape

A good planning cycle leaves:

- one canonical `PROGRAM-BLUEPRINT.md`
- execution-grade frontier specs under `specs/`
- an updated mission `README.md`
- the exact next selected target plus its passed execution-package gate state
- an explicit next verdict such as `continue_required`, `needs_user`,
  `hard_blocked`, or `replan_required`
