---
name: clarify
description: Mission-intake workflow for Codex1. Use when the user invokes $clarify, starts a new mission, or gives a vague or high-risk outcome that needs an Outcome Lock before planning.
---

# Clarify

Use this public skill to destroy planning-critical ambiguity and bootstrap the
mission package under `PLANS/<mission-id>/`.

## Owns

- mission bootstrap and mission identity selection
- `PLANS/<mission-id>/README.md`
- `PLANS/<mission-id>/MISSION-STATE.md`
- ratified `PLANS/<mission-id>/OUTCOME-LOCK.md` once lock gates pass

Bootstrap from these repo-owned templates when you create or refresh artifacts:

- `templates/mission/README.md`
- `templates/mission/MISSION-STATE.md`
- `templates/mission/OUTCOME-LOCK.md`

Deterministic backend:

- call `codex1 internal init-mission` to create or refresh the mission package
- keep reasoning and judgment in the skill, and let the command persist
  artifact writeback, gates, and the clarify closeout from that skill-owned
  truth
- see `docs/runtime-backend.md` for the machine-side contract

## Ralph Lease

Manual `$clarify` is an interactive intake workflow, not a parent Ralph loop by
default. Do not acquire a parent loop lease just because the user invoked
`$clarify`. When the lock is ratified, leave the manual handoff to `$plan`.

`$autopilot` may acquire an `autopilot_loop` lease and consume a clarified
handoff into planning, but that lease belongs to `$autopilot`, not manual
`$clarify`.

## Clarify Posture

- Clarify is an interview, not a form fill.
- Optimize for branch-reduction value per question, not field coverage.
- Ask the user only for facts or preferences that are genuinely human-owned.
- Read the repo first whenever repo evidence can answer the question better than
  the user can.
- Use the strongest ask-user mechanism available in the current Codex surface,
  but keep the interaction to one main question at a time by default.

## Ambiguity Scorecard

Keep an explicit scorecard in your head and in `MISSION-STATE.md` across these
dimensions:

- objective and intent clarity
- success proof and finish bar
- scope boundary
- non-goals
- protected surfaces and irreversible risk
- autonomy boundary
- tradeoff vetoes
- baseline facts and repo-grounded constraints
- rollout, migration, or environment limits
- decision boundaries for what Codex may choose without asking again

## Workflow

1. Resolve whether this is a new mission, a resume, or a mission-selection
   ambiguity.
2. Create or refresh the mission package before continuing.
3. Parse the user ask into a provisional destination contract:
   objective, outcome, proof bar, non-goals, constraints, protected surfaces,
   and autonomy posture.
4. Read the repo whenever that can collapse ambiguity or reveal protected
   surfaces.
5. Record provenance in `MISSION-STATE.md`:
   user-stated facts, repo-grounded facts, and Codex inferences must stay
   visibly distinct.
6. Choose the next question by leverage:
   prefer questions that collapse whole branches of planning or execution risk.
7. Ask one main high-leverage question at a time by default. Ask two or three
   only when they are tightly coupled and separating them would reduce clarity.
8. After every answer, rescore the ambiguity dimensions and update the
   provisional lock truth before asking again.
9. Run bounded feasibility probes only when repo reality may materially
   constrain the locked destination.
10. Ratify `OUTCOME-LOCK.md` only when the lock rule passes.

## Lock Rule

Lock only when all of the following are true:

- no ambiguity dimension is still scored `3`
- any remaining `2` is explicitly recorded and bounded enough not to reshape
  architecture or protected-surface handling
- success is observable
- protected surfaces are explicit
- unacceptable tradeoffs are explicit
- non-goals are explicit
- autonomy boundary is explicit
- decision boundaries are explicit enough that planning will know what Codex may
  choose autonomously
- at most three low-impact assumptions remain, and all are written down

Use `lock_posture = constrained` when the destination is ratified but bounded by
explicit feasibility or environment limits discovered during clarify.

## Feasibility Probes

Bounded feasibility probing is allowed when repo reality may prove the ask
infeasible or materially constrained.

Probe rules:

- keep probes narrow and evidence-oriented
- do not choose architecture
- do not decompose the mission into workstreams
- if uncertainty remains after bounded probing, record it honestly instead of
  inventing certainty

## Interview Rules

- Prefer why-before-how until the destination is stable enough that planning
  will not invent architecture.
- Force boundaries explicitly:
  what is out of scope, what must not be broken, and what tradeoffs are
  unacceptable.
- Revisit earlier answers when needed; do not rotate dimensions just for
  coverage.
- If the user gives a vague success bar, push until the finish condition is
  observable.
- If the mission implies risky autonomy, surface that explicitly in the lock
  rather than assuming a conservative default.

## Must Not

- drift into architecture selection or detailed sequencing
- invent fake completeness because a few fields are filled in
- flatten user intent, repo facts, and inference into one voice
- stop because the destination sounds good but still has hidden decision
  boundaries
- use `needs_user` before bounded repo reading or probing has exhausted the
  autonomous path

## Return Shape

If the lock is not ready and only the user can resolve the next blocker,
leave durable `needs_user` waiting state plus one precise question, the reason
the blocker is genuinely human-owned, and the canonical waiting request needed
for later resume.

If the lock is ready, leave:

- a refreshed mission `README.md`
- a current `MISSION-STATE.md`
- a ratified `OUTCOME-LOCK.md`
- a clean manual handoff to `$plan`

For manual `$clarify`, do not continue into planning automatically after the
lock is ratified. Leave durable handoff truth so the user can invoke `$plan`
explicitly. `$autopilot` is the workflow that may consume the clarify handoff
and continue into planning without another user command.
