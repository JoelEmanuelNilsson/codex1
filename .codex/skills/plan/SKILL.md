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

- use `codex1 internal write-blueprint` for visible blueprint/spec writeback
- use `codex1 internal compile-execution-package` and
  `codex1 internal validate-execution-package` for the next selected target
- see `docs/runtime-backend.md` for payload shape and state ownership

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
4. Keep the selected route, proof design, review contract, and packetization in
   sync after route selection. Do not carry stale pre-selection drafts forward.
5. Use `internal-orchestration` when bounded subagents materially help, and use
   `internal-replan` if planning proves that a higher layer must reopen.

## Planning Rules

- Be repo-grounded, evidence-driven, and critique-driven.
- Do not invent quota-shaped options when only one viable route survives.
- Do not let execution inherit unresolved architecture questions.
- No work item is runnable unless it has a proof story and review story.
- Compact plans are allowed for bounded low-risk work, but compact does not mean
  shallow.

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

## Must Not

- stop because one document draft exists
- hide core planning method in private code or vibes
- silently discard earlier valid evidence during replanning
- claim planning completion while the next execution target is still unpackaged

## Return Shape

A good planning cycle leaves:

- one canonical `PROGRAM-BLUEPRINT.md`
- execution-grade frontier specs under `specs/`
- an updated mission `README.md`
- an explicit next verdict: continue, needs user, hard blocked, or replan
