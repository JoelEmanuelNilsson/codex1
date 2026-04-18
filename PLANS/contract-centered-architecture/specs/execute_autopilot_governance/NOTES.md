# Spec Notes

- Mission id: `contract-centered-architecture`
- Spec id: `execute_autopilot_governance`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- `execute` is being tightened around passed-package entry, review or repair or
  replan routing, and explicit no-false-terminal discipline.
- `autopilot` is being tightened as branch routing over the same public
  contracts rather than a second hidden workflow engine.
- the final clean-frontier transition now needs to be explicit: mission-close
  review is owed before completion, even after execution goes clean.
- qualification and multi-agent docs are being aligned so the autonomy claim is
  visibly backed by parity, waiting, and child-lane proof surfaces.
- the proof receipt needs to point at the direct public-contract checks for
  mission-close routing, not imply that qualification parity alone proves the
  user-facing execute/autopilot branch contract.
- execution-graph obligations were still staying `open` even after clean spec
  reviews, which meant mission-close could drift away from graph truth. The
  runtime now reconciles passed spec-review gates back into execution-graph
  obligation satisfaction and mission-close validation now rejects unsatisfied
  blocking obligations.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
