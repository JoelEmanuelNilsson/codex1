# ralph-control-loop-boundary

This file is summary-only. If it ever drifts from the canonical artifacts below,
the canonical artifacts win.

## Snapshot

- Mission id: `ralph-control-loop-boundary`
- Current phase: `mission_close`
- Current verdict: `complete`
- Next recommended action: Mission-close review passed; the mission may stop as complete.
- Current blocker: Mission close conditions are satisfied.

## Start Here

1. Read `OUTCOME-LOCK.md` for destination truth.
2. Read `PROGRAM-BLUEPRINT.md` for route truth.
3. Read `specs/control_loop_qualification/SPEC.md` if execution is active.
4. Check `REVIEW-LEDGER.md` and `REPLAN-LOG.md` when they exist.

## Objective Summary

Codex1 must make Ralph continuation scoped instead of global: Ralph may enforce continuation only for the main parent/orchestrator thread while an explicit loop workflow is active, and must not block normal user communication or any subagent from stopping.

## Active Frontier

- Selected target: `mission:ralph-control-loop-boundary`
- Why it is next: Recorded review bundle 7c054b93-c388-4d8a-b1c7-2097564d4c10 with 0 blocking finding(s)
- Expected proof, review, or package gate: Next governed phase: `complete`.

## Current Risks Or Blockers

- Mission close conditions are satisfied.
- Mission close conditions are satisfied.

## Canonical Artifacts

- `MISSION-STATE.md` is canonical only for live clarify worksheet state.
- `OUTCOME-LOCK.md` is canonical for destination truth.
- `PROGRAM-BLUEPRINT.md` is canonical for route truth.
- `specs/*/SPEC.md` is canonical for one bounded execution slice.
- `REVIEW-LEDGER.md` is canonical for readable review history.
- `REPLAN-LOG.md` is canonical for readable non-local replan history.
