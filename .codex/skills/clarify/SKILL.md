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

## Workflow

1. Resolve whether this is a new mission or a resume of an existing one.
2. Create or refresh the mission package before asking the next question.
3. Parse the user ask into provisional lock fields.
4. Read the repo whenever that reduces technical ambiguity or reveals protected
   surfaces.
5. Score ambiguity across these dimensions: objective clarity, success proof,
   protected surfaces, tradeoff vetoes, scope boundary, autonomy boundary,
   baseline facts, and rollout or migration constraints.
6. Choose the next question by branch-reduction value, not by missing-field
   order.
7. Ask one main high-leverage question at a time by default. Ask two or three
   only when the questions are tightly coupled and separating them would reduce
   clarity.
8. Keep `MISSION-STATE.md` current with provenance for user-stated facts,
   repo-grounded facts, and Codex inferences.
9. Ratify `OUTCOME-LOCK.md` only when the lock rule passes.

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

## Must Not

- drift into architecture selection or detailed sequencing
- invent fake completeness because a few fields are filled in
- flatten user intent, repo facts, and inference into one voice
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
- a clean handoff to `$plan`
