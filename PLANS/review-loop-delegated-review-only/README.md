# review-loop-delegated-review-only

This file is summary-only. If it ever drifts from the canonical artifacts below,
the canonical artifacts win.

## Snapshot

- Mission id: `review-loop-delegated-review-only`
- Current phase: `review`
- Current verdict: `repair_required`
- Next recommended action: Review did not pass cleanly; address 2 blocking review finding(s) or reconcile verdict blocked.
- Current blocker: Blocking gate remains open: `review-loop-delegated-review-only:execution_package:spec:delegated_review_qualification_guard:c1db1c9d-8eb8-435e-b0ec-d89f5a12461f:spec:delegated_review_qualification_guard`.

## Start Here

1. Read `OUTCOME-LOCK.md` for destination truth.
2. Read `PROGRAM-BLUEPRINT.md` for route truth.
3. Read `specs/reviewer_output_inbox_contract/SPEC.md` if execution is active.
4. Check `REVIEW-LEDGER.md` and `REPLAN-LOG.md` when they exist.

## Objective Summary

Correct Codex1's review-loop product contract so review judgment is always delegated to reviewer agent roles. The parent/orchestrator may explore context, prepare snapshots and briefs, select reviewer profiles, spawn reviewer agents, aggregate child outputs, detect contamination, route repair/replan, and record parent-owned gate outcomes, but the parent must not itself perform code review, spec review, intent review, integration review, or mission-close review judgment.

## Active Frontier

- Selected target: `spec:reviewer_output_inbox_contract`
- Why it is next: Recorded review bundle 90ca57cc-ee7e-4857-9772-2e41c6b12f5e with 2 blocking finding(s)
- Expected proof, review, or package gate: Next governed phase: `execution`.

## Current Risks Or Blockers

- Blocking gate remains open: `review-loop-delegated-review-only:execution_package:spec:delegated_review_qualification_guard:c1db1c9d-8eb8-435e-b0ec-d89f5a12461f:spec:delegated_review_qualification_guard`.
- Blocking gate remains open: `review-loop-delegated-review-only:execution_package:mission:review-loop-delegated-review-only:534a3c8b-f157-4b86-b081-da7ee0bfec30:mission:review-loop-delegated-review-only`.

## Canonical Artifacts

- `MISSION-STATE.md` is canonical only for live clarify worksheet state.
- `OUTCOME-LOCK.md` is canonical for destination truth.
- `PROGRAM-BLUEPRINT.md` is canonical for route truth.
- `specs/*/SPEC.md` is canonical for one bounded execution slice.
- `REVIEW-LEDGER.md` is canonical for readable review history.
- `REPLAN-LOG.md` is canonical for readable non-local replan history.
