# Spec Notes

- Mission id: `review-lane-role-contract`
- Spec id: `review_loop_skill_surface`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added `.codex/skills/review-loop/SKILL.md` as the canonical parent-owned
  review orchestration skill and removed `.codex/skills/review/SKILL.md`.
- Updated managed support-surface expectations so required/managed public
  skills include `review-loop` and no longer include `review`.
- Updated AGENTS scaffolds, execute/autopilot routing text, runtime backend
  docs, PRD operator references, and qualification/runtime test fixtures to
  use `$review-loop`.
- Direct reviewer agents remain prompt-only findings roles; this slice does not
  implement deeper reviewer profiles or Ralph child-lane isolation.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
