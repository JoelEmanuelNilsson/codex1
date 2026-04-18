# Spec Notes

- Mission id: `review-lane-role-contract`
- Spec id: `review_loop_orchestration`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added deterministic review-loop decision proof in qualification code for the
  three parent branches: clean continue, non-clean repair before cap, and
  sixth non-clean replan.
- The proof model treats P0/P1/P2 as blocking and P3 as non-blocking by
  default, matching the `$review-loop` skill contract.
- The decision helper deduplicates blocking root causes while preserving the
  blocking finding count.
- Full child spawning/orchestration remains skill-led; this slice proves the
  parent branch semantics without introducing a hidden review engine.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
