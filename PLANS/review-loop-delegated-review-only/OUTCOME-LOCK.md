---
artifact: outcome-lock
mission_id: review-loop-delegated-review-only
root_mission_id: review-loop-delegated-review-only
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: review-loop-delegated-review-only
---
# Outcome Lock

## Objective

Correct Codex1's review-loop product contract so review judgment is always delegated to reviewer agent roles. The parent/orchestrator may explore context, prepare snapshots and briefs, select reviewer profiles, spawn reviewer agents, aggregate child outputs, detect contamination, route repair/replan, and record parent-owned gate outcomes, but the parent must not itself perform code review, spec review, intent review, integration review, or mission-close review judgment.

## Done-When Criteria

- `$review-loop` explicitly states that substantive review judgment is performed only by reviewer agents, never by the parent/orchestrator.
- Parent/orchestrator responsibilities are limited to orchestration, evidence preparation, reviewer selection, output aggregation, contamination detection, routing, and durable writeback.
- Direct parent self-review is forbidden for spec reviews, code bug/correctness reviews, integration reviews, and mission-close reviews.
- Review outcomes must cite reviewer-agent outputs or a contamination/replan reason; they cannot be justified by parent-only code/spec judgment.
- The reviewer-lane capability guard remains intact: child reviewers still cannot mutate mission truth or clear gates.
- Docs, skills, and qualification/test surfaces no longer imply that the parent may locally review a slice.

## Success Measures

- Public `$review-loop` docs distinguish parent orchestration from reviewer-agent judgment with no local-review loophole.
- Any future review-loop execution should launch reviewer agents for actual review judgment.
- Parent-owned writeback remains valid only after aggregating reviewer-agent outputs or recording an explicit contaminated/replan path.
- Tests or qualification checks catch wording or behavior that permits parent self-review.

## Protected Surfaces

- `$review-loop` public skill semantics.
- `internal-orchestration` reviewer child contract.
- Review truth/evidence snapshot flow.
- `record-review-outcome`, review ledgers, gates, closeouts, and mission-close terminal truth.

## Unacceptable Tradeoffs

- Do not allow the parent to self-approve because a slice is small, high-context, or about reviewer machinery.
- Do not weaken child reviewer findings-only and no-mutation constraints.
- Do not replace reviewer agents with parent local judgment.
- Do not remove parent responsibility for orchestration, aggregation, contamination detection, routing, and writeback.

## Non-Goals

- Redesign all review profiles from scratch.
- Remove `$review-loop`.
- Allow child reviewers to record outcomes or clear gates.
- Solve platform-level read-only sandboxing beyond the existing snapshot/guard model.

## Autonomy Boundary

- Codex may decide implementation details for docs, tests, and runtime/qualification checks that enforce delegated-review-only semantics.
- Codex must ask before creating any exception where parent review judgment can substitute for reviewer-agent judgment.

## Locked Field Discipline

The fields above are locked. Change only through explicit reopen.

## Baseline Current Facts

- Current `$review-loop` documentation says the parent owns orchestration and writeback and uses child reviewers, but does not explicitly say the parent can never be the reviewer.
- Recent execution used parent local review while repairing reviewer-lane isolation, which contradicts the user's intended product contract.
- Reviewer capability boundary now provides snapshots and contamination detection, so delegated reviewer agents can be used more safely.

## Reopen Conditions

- Reopen only if native Codex cannot create reviewer agents for review judgment and the user explicitly authorizes a different review model.

## Provenance

### User-Stated Intent

- The parent is never to do review.
- The parent can explore things and think about things, but should not do code review.
- `$review-loop` is explicitly to orchestrate reviewer agent roles of different sorts.

### Repo-Grounded Facts

- `$review-loop` already defines child reviewer lanes and parent-owned writeback, but leaves room for parent local review by omission.
- `reviewer-lane-capability-boundary` added snapshot/guard mechanics that support safer child reviewer delegation.

### Codex Clarifying Synthesis

- The missing contract is not reviewer safety alone; it is reviewer-role authority: review judgment belongs to reviewer agents, while the parent owns orchestration and durable truth.
