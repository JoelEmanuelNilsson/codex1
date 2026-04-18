# Spec Notes

- Mission id: `review-lane-role-contract`
- Spec id: `ralph_review_lane_isolation`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added `ChildLaneRole::FindingsOnlyReviewer` to child lane expectations and
  reconciliation output so review-lane metadata is durable and inspectable.
- Added optional Stop-hook lane metadata (`laneRole` / `childLaneKind`) so
  findings-only reviewer lanes can return bounded review payloads without being
  blocked by parent mission gates.
- Parent/controller Stop-hook behavior remains unchanged: parent mission gates
  still block parent progress.
- Added an integration test proving a parent Stop hook blocks on an open review
  gate while a findings-only reviewer lane in the same repo is allowed to
  return.
- Documented the lane-role contract in `MULTI-AGENT-V2-GUIDE.md` and
  `docs/runtime-backend.md`.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
