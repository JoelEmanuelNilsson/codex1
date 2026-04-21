# Round 8 Meta-Review

## Verdict

Round 8 found real remaining issues. After finding review:

- P0: 0
- P1: 6
- P2: 11
- P3 / cleanup: 2

No round-8 candidate was clean enough to count as a clean round. The clean-round counter remains 0.

## Finding Review

| Finding | Meta-review verdict |
| --- | --- |
| F01 symlinked mission/review directories allow CLI-owned writes outside `PLANS/<mission>` | Accepted P1. This is a production write-side containment escape, not merely a missing symlink-read test. Reject symlinked mission roots and symlinked artifact parents before CLI-owned writes. |
| F02 missing `definitions` / `resolved_questions` pass outcome ratification | Accepted P2, downgraded from P1. It violates the ratification contract but does not corrupt existing data or create an unrecoverable/security failure. |
| F03 review task can omit reviewed targets from `depends_on` | Accepted P2. `plan check` must reject review tasks whose direct dependencies do not include every `review_target.tasks` entry. Add defense so review tasks are not scheduled as ordinary work. |
| F04 `plan waves` reports downstream wave while upstream dependency is in progress | Accepted P2. Repair with shared/actionable wave readiness semantics. |
| F05 `review start` masks stale `--expect-revision` behind `PLAN.yaml` parsing | Accepted P2. Incomplete round-7 stale-writer repair; revision must be checked before plan/domain preflight. |
| F06 superseded live-DAG projection disagrees across `plan waves`, `status`, and `task next` | Accepted P2. Merge R01/R14/R16 into this item. Keep F16 as a stronger lifecycle regression in the same repair area. |
| F07 `review start` erases an active dirty review | Accepted P1. Current dirty review truth can be cleared without repair by restarting and then recording clean. |
| F08 late dirty review records become current blockers | Accepted P1. Non-current categories must be audit-only and must not replace current review truth or block readiness. |
| F09 stale mission-close dirty record reopens after clean | Accepted P1. Revalidate close-review recordability under the state lock. |
| F10 concurrent mission-close stale writers overwrite committed findings artifact | Accepted P1. Related to F09 but distinct: rejected writers must not write the committed artifact path. |
| F11 Ralph fails open on ambiguous multi-mission status errors | Accepted P2. Related to round-7 multi-mission status repair, but hook-side behavior remains wrong. |
| F12 `$execute` rejects valid `$autopilot` / `status` handoffs | Accepted P1. Repair `$execute` preconditions so executable/repair next actions can activate the loop and proceed. |
| F13 `verify-installed` can pass without verifying concrete installed binary or `/tmp doctor` | Accepted P2. Narrow title to the remaining issue: bind smoke to installed path and run doctor from the `/tmp` smoke environment. |
| F14 `install-local` breaks on install dirs with spaces | Verified, downgraded to P3. Fix opportunistically only if touching the Makefile for F13. |
| F15 close can complete after required proof file is deleted | Accepted P2. Close readiness must revalidate proof artifacts for completed non-superseded tasks. |
| F16 replanning reviewed work strands orphan review task | Accepted P2. Merge with live-DAG/supersession repair area but keep a distinct close-blocking regression case. |
| F17 review packet drops stored absolute proof paths | Accepted P2. Packet proofs must come from recorded `TaskRecord.proof_path`, with conventional fallback for legacy states. |
| F18 replan docs scalar/array mismatch | Verified, downgraded to P3. CLI is internally consistent; update docs only as cleanup if already touching CLI docs. |

## Accepted Repair Set

1. Add write-side mission containment for symlinked mission roots and artifact directories.
2. Enforce all required OUTCOME fields in `outcome check` / `ratify`.
3. Tighten review DAG validation and prevent review tasks from being scheduled as normal work.
4. Unify actionable wave/live-DAG behavior across `plan waves`, `status`, and `task next`.
5. Fix `review start` stale revision precedence.
6. Preserve current review truth: `review start` must not erase dirty current reviews, and non-current review records must be audit-only.
7. Make mission-close review recording concurrency-safe: locked precondition revalidation and no stale writer artifact overwrite.
8. Make Ralph handle ambiguous multi-mission status safely.
9. Fix `$execute` skill handoff preconditions and loop activation.
10. Bind install verification to the concrete installed binary and run the documented `/tmp` doctor/init/status smoke.
11. Revalidate proof artifacts at close readiness and use recorded proof paths in review packets.
12. Fix superseded review-task lifecycle after replan so old review tasks do not strand close.

## Dropped / Downgraded

- F14 whitespace install directory is P3. It is cheap to fix while editing the Makefile, but it is not a P0/P1/P2 blocker.
- F18 replan scalar/array docs mismatch is P3. It is a docs cleanup, not an accepted P0/P1/P2 repair.

## Repair Notes

F04, F06, and F16 should be repaired together if possible. The intended rule needs to be explicit: live executable projections should not silently diverge when a live task depends on a superseded task. Either all public surfaces fail closed or all public surfaces use the same filtered live DAG. The current `plan waves` fail-closed behavior is the safest starting point.

F07 and F08 should be repaired together. Current review truth must be distinguishable from audit-only late/stale records.

F09 and F10 should be repaired together. A close-review dirty record needs one winning transaction that both validates current state and owns the committed artifact path.

