# Round 7 Meta-Review

## Verdict

Round 7 is directionally useful, but it over-counts duplicates and overstates one priority.

Adjusted outcome:

- P0: 0
- P1: 0
- P2: 10 unique repair items
- P3/doc cleanup: 1

Main changes:

- Downgrade P1-1 to P2. It is a security-relevant test gap, not a verified production bug.
- Merge P2-3 and P2-4 into P2-1. They are concrete examples of the same stale-writer ordering defect.
- Merge P2-5 into P2-6. Both are symptoms of inconsistent live-DAG/superseded-task projection.
- Downgrade P2-8 to P3. The skill metadata summary is incomplete, but the skill body points to the complete `references/outcome-shape.md`, and CLI ratification still catches missing fields.

## Finding Review

| Finding | Meta-review |
| --- | --- |
| P1-1 absolute-path and symlink containment branches under-tested | Valid test gap, downgrade to P2. The helper has distinct absolute, parent-component, and canonical containment branches, but current tests cover only relative traversal. No current production failure was shown. |
| P2-1 stale-writer conflicts masked by task/review gates | Valid P2. `task start`, `task finish`, `review start`, and `review record` can return domain errors before enforcing `--expect-revision`. |
| P2-2 `plan check` masks stale writers behind plan validation | Valid P2. `plan check` parses/validates `PLAN.yaml` before loading state and checking `--expect-revision`. |
| P2-3 `review record` masks stale writers behind findings-file checks | Merge into P2-1. This is the findings-file instance of the same stale-writer ordering defect. |
| P2-4 `task finish` masks stale writers behind proof checks | Merge into P2-1. This is the proof-file instance of the same stale-writer ordering defect. |
| P2-5 review readiness does not drop superseded targets | Valid symptom, merge into P2-6. `status` can surface a review whose targets are no longer live, while `task next` filters to awaiting-review targets. |
| P2-6 superseded tasks skew wave derivation | Valid P2. `plan waves`, `status`, and `task next` still apply different rules after supersession. |
| P2-7 clean close-review dry-run previews stale dirty count | Valid P2. Dry-run emits the current mission-close dirty counter, while the wet clean path resets it to 0. |
| P2-8 clarify skill omits required OUTCOME fields in summary | Downgrade to P3. The metadata summary omits `status` and `definitions`, but the workflow itself references the complete required-field file. Worth fixing, not P2. |
| P2-9 CLI reference uses nonexistent replan reason | Valid P2. The documented `six_consecutive_dirty` reason is rejected by the implemented allowed-reason set. |
| P2-10 bare `status` hides ambiguous mission resolution | Valid P2. Current `status` collapses all bare `MissionNotFound` cases into `reason: no_mission`, despite the contract saying multiple candidates require disambiguation. Note prior round-2 text observed the graceful behavior, but the current contract is the stronger source. |
| P2-11 verify target does not run documented `/tmp` smoke | Valid P2. `verify-installed` does not exercise installed-binary `init` and `status` from `/tmp`, even though docs call that the critical verification. |
| P2-12 `unknown_side_effects` blocker coverage missing | Valid P2 test gap. `plan waves` covers it, but status/task-next coverage only exercises exclusive-resource blockers. |
| P2-13 close-review staging failure path under-tested | Valid P2 test gap. The artifact-before-state invariant is important and should have a failure-path test that proves state remains unchanged. |

## Accepted Repair Set

1. Fix stale-writer precedence in task/review mutators: P2-1, including merged P2-3 and P2-4.
2. Fix stale-writer precedence in `plan check`: P2-2.
3. Unify live-DAG/superseded-task projection across `plan waves`, `status`, and `task next`: P2-6, including merged P2-5.
4. Fix mission-close clean dry-run preview to report `consecutive_dirty: 0`: P2-7.
5. Fix the replan reason examples and success shape in CLI docs: P2-9.
6. Distinguish true no-mission status fallback from ambiguous multi-mission resolution: P2-10.
7. Extend installed verification to run the documented `/tmp` `init` and `status` smoke: P2-11.
8. Add absolute-path and symlink escape tests for mission ids/spec containment: downgraded P1-1.
9. Add status/task-next `unknown_side_effects` blocker tests: P2-12.
10. Add close-review artifact-staging failure test with unchanged state assertions: P2-13.
11. Cleanup only: add `status` and `definitions` to the clarify skill metadata summary or replace the inline list with a pointer to `references/outcome-shape.md`: downgraded P2-8.

## Repair Order

1. Stale-writer ordering first: fix P2-1 and P2-2 together. This is a cross-command contract issue and should get focused regression tests for the masked proof/findings/plan-validation cases.
2. Live-DAG/supersession consistency next: fix P2-6 and P2-5 together. Prefer one shared readiness/wave rule instead of patching `status` and `task next` independently.
3. Close-review consistency: fix P2-7, then add the P2-13 staging-failure test while the close-review surface is fresh.
4. Public workflow/docs correctness: fix P2-10, P2-11, and P2-9.
5. Test-coverage hardening: add the downgraded P1-1 path tests and P2-12 `unknown_side_effects` tests.
6. P3 cleanup last: clarify skill metadata summary.

## Notes

P2-1, P2-3, and P2-4 should not remain separate repair tickets. Keep the concrete repros as test cases, but track one stale-writer ordering fix.

P2-5 and P2-6 should be repaired as a single live-DAG design issue. The intended behavior needs one explicit rule for whether live tasks may depend on superseded tasks, then all three public projections should follow it.

For P2-10, avoid string-matching `MissionNotFound` messages if possible. A distinct ambiguous-mission error variant or structured context would make `status` able to preserve the graceful true-no-mission fallback without hiding ambiguity.

For P1-1's downgraded coverage item, include both CLI-level tests and direct helper coverage if practical: absolute mission id, absolute spec path, and a mission-local symlink that canonicalizes outside `PLANS/<mission>/`.
