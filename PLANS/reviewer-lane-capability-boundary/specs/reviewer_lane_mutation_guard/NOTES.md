# Spec Notes

- Mission id: `reviewer-lane-capability-boundary`
- Spec id: `reviewer_lane_mutation_guard`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added `ReviewTruthSnapshot` and `capture-review-truth-snapshot` so the parent
  review loop can capture visible and hidden mission truth before launching
  findings-only child reviewer lanes.
- `record-review-outcome` now accepts an optional `review_truth_snapshot` and
  rejects writeback with `reviewer_lane_truth_mutation_detected` if the mission
  tree changed before parent-owned review writeback.
- Added regression tests for contaminated child-lane mutation rejection and
  clean parent-owned snapshot-backed review writeback.
- Tightened `record-review-outcome` at the CLI boundary so writeback now
  requires a parent-owned `review_truth_snapshot`; this prevents a child
  reviewer from bypassing the guard by omitting the snapshot.
- Updated `$review-loop`, `internal-orchestration`, runtime backend docs, and
  the Multi-Agent guide to require the parent-owned snapshot guard.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
