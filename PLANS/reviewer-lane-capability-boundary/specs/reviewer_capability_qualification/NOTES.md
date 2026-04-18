# Spec Notes

- Mission id: `reviewer-lane-capability-boundary`
- Spec id: `reviewer_capability_qualification`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Added a `reviewer_capability_boundary` qualification gate that proves the
  exact child-review failure mode is release-blocking.
- The gate runs an isolated contaminated review wave and requires
  `reviewer_lane_truth_mutation_detected` before considering the proof clean.
- The same gate validates a frozen review evidence snapshot and proves clean
  parent-owned snapshot-backed review writeback still passes.
- Qualification docs now list the reviewer capability boundary as an explicit
  qualification gate.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
