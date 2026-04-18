---
artifact: outcome-lock
mission_id: reviewer-lane-capability-boundary
root_mission_id: reviewer-lane-capability-boundary
parent_mission_id: null
version: 1
lock_revision: 1
status: locked
lock_posture: unconstrained
slug: reviewer-lane-capability-boundary
---
# Outcome Lock

## Objective

Fix the serious review-lane isolation failure observed during the manual-clarify-handoff review: findings-only reviewer lanes violated their prompt contract, used mission tooling, cleared gates, and advanced mission-close truth. Codex1 must make reviewer lanes capability-safe enough that child reviewers cannot silently mutate mission truth, clear gates, record review outcomes, or terminalize a mission.

## Done-When Criteria

- Findings-only reviewer lanes review frozen, bounded evidence rather than live mutable mission truth whenever practical.
- Parent `$review-loop` remains the only authority that aggregates child outputs and calls `record-review-outcome`.
- Parent review-loop records a pre-review mission-truth snapshot and rejects or blocks if child reviewer execution changes gates, closeouts, state, ledgers, specs, receipts, or mission-close artifacts outside the parent-owned writeback phase.
- `record-review-outcome` and review-loop qualification prove that child-lane outputs alone cannot clear gates or terminalize a mission.
- The Stop-hook child-lane bypass remains limited to allowing child result delivery; it must not become permission to mutate mission truth.
- The docs and skills make the enforcement model explicit enough that future reviewers are not asked to self-police an unenforced contract.

## Success Measures

- A regression test simulates or detects a child reviewer attempting mission-truth mutation and proves the parent blocks or rejects the contaminated review wave.
- A clean findings-only child output is accepted only after parent-owned aggregation and parent-owned `record-review-outcome`.
- Existing review-loop, stop-hook, mission artifact, gate, and closeout validation remain green.
- Future review agents should be unable to repeat the exact incident without producing a visible blocker.

## Protected Surfaces

- `$review-loop`, `internal-orchestration`, and findings-only reviewer role semantics.
- `record-review-outcome`, review bundles, gates, closeouts, state, review ledger, and mission-close terminal truth.
- Ralph Stop-hook behavior for parent versus child lanes.
- Existing package, review-bundle, and mission-close validation guarantees.

## Unacceptable Tradeoffs

- Do not rely on prompt-only reviewer discipline as the only protection.
- Do not let child reviewers directly clear gates, record review outcomes, or terminalize missions.
- Do not break the legitimate ability for child reviewers to return `NONE` or structured findings while the parent mission has an open gate.
- Do not replace native Codex subagents with an external wrapper runtime.
- Do not hide review authority in private chat state that cannot be validated from repo artifacts.

## Non-Goals

- Redesign all planning quality.
- Replace `$review-loop` or rename public skills again.
- Solve platform-level sandboxing that the Codex native tool surface does not expose.
- Require reviewers to perform write-capable repairs.

## Autonomy Boundary

- Codex may decide exact implementation details for reviewer evidence snapshots, mutation guards, review-wave manifests, authority tokens, or deterministic validation hooks as long as parent-owned review writeback remains the only accepted mission-truth mutation path.
- Codex must ask before allowing child reviewers to mutate mission truth, lowering P0/P1/P2 cleanliness semantics, removing `$review-loop`, or adopting an external babysitter runtime as the primary solution.

## Locked Field Discipline

The fields above are locked. Change only through explicit reopen.

## Baseline Current Facts

- The prior `review-lane-role-contract` mission documented findings-only child reviewers, but the live incident proved prompt-only discipline was insufficient.
- `stop-hook` currently bypasses parent gate blocking for inputs that identify findings-only reviewer lanes.
- `record-review-outcome` is available to the same local repo process and currently does not prove that the caller is the parent review-loop rather than a child lane.
- A child reviewer can contaminate mission truth before the parent aggregates findings unless the parent has a mutation snapshot or stronger capability boundary.

## Rollout Or Migration Constraints

- The fix should be native-Codex compatible and repo-artifact driven.
- If full prevention is impossible with current platform primitives, detection and rejection must be explicit, durable, and release-blocking rather than best-effort logging.

## Remaining Low-Impact Assumptions

- The first execution slice can focus on deterministic mutation detection and parent-owned authority checks before considering heavier evidence-export ergonomics.
- Reviewers may still receive enough read context to produce useful findings, but the parent must not trust the workspace remained untouched without validation.

## Reopen Conditions

- Reopen if native reviewer lanes cannot be bounded without making review unusably weak.
- Reopen if the only viable route requires platform-level permission controls outside this repo.

## Provenance

### User-Stated Intent

- User wants reviewer lanes to be findings-only and not invoke parent review workflows.
- User wants review loops to stop recurring failure patterns and replan when architecture is wrong.

### Repo-Grounded Facts

- Existing review-loop skill forbids child mutation, but live execution showed subagents still cleared gates and mission-close state.
- Existing Ralph child-lane isolation proves child Stop-hook delivery, not mutation prevention.

### Codex Clarifying Synthesis

- The product needs a real capability boundary plus contamination detection, not more prompt wording.
