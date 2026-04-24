# 08 State, Status, And Graph Contract

This file is canonical for durable state, status projection, revisions, and
graph/wave derivation.

The goal is a deterministic substrate that Codex can reason over without
building a workflow daemon.

## Durable Files

Every durable mission lives under:

```text
PLANS/
  ACTIVE.json
  <mission-id>/
    OUTCOME.md
    PLAN.yaml
    STATE.json
    EVENTS.jsonl
    specs/
    reviews/
    CLOSEOUT.md
```

Optional task-local artifacts:

```text
specs/T1/SPEC.md
specs/T1/PROOF.md
reviews/R-001.json
reviews/findings.yaml
```

## Ownership Of Truth

Keep truth split cleanly:

| File | Owns | Does Not Own |
| --- | --- | --- |
| `PLANS/ACTIVE.json` | Repo-local default mission selection for read/status/Ralph paths | Mission phase, task state, loop truth, review truth |
| `OUTCOME.md` | User-ratified destination, acceptance criteria, constraints | Runtime task status |
| `PLAN.yaml` | Static route: mode, tasks/steps, dependencies, review requirements, proof requirements | Mutable progress |
| `STATE.json` | Mutable operational state: phase, loop, task status, review state, revision | Plan structure |
| `EVENTS.jsonl` | Append-only audit of mutations | Replay authority in v1 |
| `reviews/*` | Raw reviews and adjudicated findings | Mission state by itself |
| `CLOSEOUT.md` | Final summary and proof | Active mission truth |

Do not duplicate mutable truth between `PLAN.yaml` and `STATE.json`.

If `PLAN.yaml` includes initial status fields for readability, they are template
defaults only. Once the plan is locked, mutable status belongs in `STATE.json`.

## Schema Versions

Every durable artifact must include `schema_version`.

Recommended values:

```text
codex1.outcome.v1
codex1.plan.v1
codex1.state.v1
codex1.event.v1
codex1.review.v1
codex1.finding.v1
codex1.spec.v1
codex1.proof.v1
codex1.closeout.v1
codex1.status.v1
codex1.error.v1
```

Markdown files should use frontmatter:

```yaml
---
schema_version: codex1.outcome.v1
mission_id: codex1-rebuild
---
```

JSON/YAML files should use a top-level field:

```json
{
  "schema_version": "codex1.state.v1"
}
```

Unsupported schema versions should fail normal CLI writes, but Ralph should
fail open because a broken guardrail must not trap the session.

## State Revision

`STATE.json.revision` is the ordering authority.

Every successful mutating command increments it by exactly 1.

Timestamps are allowed as metadata, but they do not determine freshness.

`EVENTS.jsonl` should normally contain exactly one event for each committed
state revision. If `STATE.json.revision` is ahead of the latest event revision,
state remains authoritative and `doctor` should report audit drift.

Mutating command protocol:

```text
load STATE.json
validate --expect-revision if provided
parse relevant artifacts
validate transition preconditions
acquire mission-local lock
re-read STATE.json under lock
re-check revision and preconditions
write new STATE.json atomically
append one EVENTS.jsonl line for the new revision
return JSON with state_revision
```

Mission-local locking:

- Use one lock path per mission: `PLANS/<mission-id>/.codex1.lock`.
- Acquire the lock with an atomic create operation. Do not check existence and
  then write; that is a race.
- Lock file contents should include the command, process id when available,
  hostname when available, and `started_at`.
- If the lock already exists, fail with a stable retryable `MISSION_LOCKED`
  error unless an explicit repair command proves it is stale.
- Stale-lock repair must be conservative, visible, and never automatic inside
  Ralph.

Atomic writes:

- Write replacement state to a temporary file in the mission directory.
- Flush/fsync when practical for the target platform.
- Atomically rename the temporary file over `STATE.json`.
- Append the matching `EVENTS.jsonl` line after the state commit.
- If `STATE.json` commits but event append fails, state remains authoritative;
  return a warning and let `doctor` report audit drift. Do not roll back by
  guessing from partially written audit state.

Stable revision conflict error:

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "REVISION_CONFLICT",
  "message": "Expected state revision 17 but current revision is 18.",
  "retryable": true,
  "current_revision": 18
}
```

## Event Semantics

`EVENTS.jsonl` is audit in v1, not replay authority. Do not reconstruct mission
truth from events unless a future version explicitly implements and tests replay.

One successful mutation writes one event:

```json
{
  "schema_version": "codex1.event.v1",
  "mission_id": "codex1-rebuild",
  "event_id": "codex1-rebuild:18",
  "revision_before": 17,
  "revision_after": 18,
  "type": "task_started",
  "actor": "cli",
  "subject": { "kind": "task", "id": "T2" },
  "changes": {
    "tasks.T2.status": ["pending", "in_progress"]
  }
}
```

Do not build event replay in v1. That is a separate database project.

## State Shape

Recommended `STATE.json` skeleton:

```json
{
  "schema_version": "codex1.state.v1",
  "mission_id": "codex1-rebuild",
  "revision": 17,
  "phase": "execute",
  "planning_mode": "graph",
  "planning_level": {
    "requested": "medium",
    "effective": "hard"
  },
  "outcome": {
    "ratified": true,
    "ratified_revision": 2
  },
  "plan": {
    "locked": true,
    "locked_revision": 5,
    "plan_digest": "sha256:...",
    "supersedes": []
  },
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "steps": {},
  "tasks": {
    "T1": {
      "status": "complete",
      "started_revision": 8,
      "finished_revision": 10,
      "proof": "specs/T1/PROOF.md",
      "superseded_by": null
    },
    "T2": {
      "status": "pending",
      "started_revision": null,
      "finished_revision": null,
      "proof": null,
      "superseded_by": null
    }
  },
  "reviews": {
    "RB-T2-1": {
      "target": {
        "tasks": ["T2"],
        "boundary_revision": 14
      },
      "state": "not_started",
      "repair_round": 0,
      "max_repair_rounds": 2,
      "latest_review_id": null,
      "accepted_blocking_count": 0
    }
  },
  "replan": {
    "required": false,
    "reason": null,
    "boundary_id": null,
    "supersedes": []
  },
  "close": {
    "state": "not_ready",
    "requires_mission_close_review": true,
    "boundary_revision": null,
    "latest_review_id": null,
    "passed_revision": null,
    "closeout_path": "CLOSEOUT.md"
  },
  "terminal": {
    "complete": false,
    "completed_revision": null
  }
}
```

Keep statuses finite and boring.

Task/step statuses:

```text
pending
in_progress
proof_submitted
review_pending
review_clean
complete
superseded
cancelled
```

Review boundary states:

```text
not_started
review_open
triage_required
repair_required
repair_done
passed
replan_required
superseded
```

Close states:

```text
not_ready
ready_for_mission_close_review
mission_close_review_open
mission_close_review_passed
close_complete_ready
terminal_complete
```

Do not add a normal `blocked_external` state. Do not add a top-level
`validation_required` state.

## Post-Lock Verdicts

After mission lock, use a small verdict set:

```text
inactive
paused
invalid_state
continue_required
replan_required
close_required
complete
```

`needs_user` is not part of normal post-lock execution. Clarification happens
before mission lock, or through explicit `$interrupt`.

`blocked_external` is intentionally absent. Ordinary engineering trouble should
be resolved by continuing, repairing, or replanning.

`validation_required` is intentionally absent. Proof and validation are part of
`finish_task`, `review`, and `close` checks.

## Status JSON Contract

`codex1 status --json` is the projection used by skills, the main thread, and
Ralph.

It should be small enough to read, but complete enough to avoid guessing.

Recommended shape:

```json
{
  "ok": true,
  "schema_version": "codex1.status.v1",
  "mission_id": "codex1-rebuild",
  "mission_root": "/abs/path/PLANS/codex1-rebuild",
  "state_revision": 18,
  "plan_digest": "sha256:...",
  "verdict": "continue_required",
  "phase": "execute",
  "planning_mode": "graph",
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "next_action": {
    "kind": "run_wave",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "wave_id": "W2",
    "tasks": ["T2", "T3"]
  },
  "ready_steps": [],
  "ready_tasks": ["T2", "T3"],
  "ready_wave": {
    "wave_id": "W2",
    "tasks": ["T2", "T3"],
    "parallel_safe": true,
    "parallel_blockers": []
  },
  "reviews": {
    "pending_boundaries": [],
    "accepted_blocking_count": 0
  },
  "replan": {
    "required": false,
    "reason": null
  },
  "close": {
    "ready": false,
    "required": false,
    "requires_mission_close_review": true
  },
  "stop": {
    "allow": false,
    "reason": "block_active_graph_wave",
    "mode": "strict",
    "message": "Codex1 says required work remains: run wave W2: T2, T3.\nContinue that now, or use $interrupt / codex1 loop pause to stop intentionally.\nIf this is a false positive, explain briefly and stop; Ralph will not block again in this turn."
  }
}
```

Ralph should consume only `stop` plus the next-action ownership fields needed to
verify that a block is safe.

## Next Action Contract

Every status projection should include at most one next action for Ralph
purposes.

Required fields:

```json
{
  "kind": "run_wave",
  "owner": "codex",
  "required": true,
  "autonomous": true
}
```

Allowed post-lock kinds:

```text
run_step
run_wave
finish_task
run_review
triage_review
repair
replan
close_review
close_complete
explain_and_stop
none
```

If a task needs proof, the next action is `finish_task`, not
`validation_required`.

If a review boundary is still dirty after repair budget, the next action is
`replan`, not `needs_user`.

If there is no autonomous next action, Ralph allows stop.

Use `explain_and_stop` when Codex cannot continue autonomously and should simply
explain why:

```json
{
  "kind": "explain_and_stop",
  "owner": "codex",
  "required": false,
  "autonomous": false,
  "reason": "missing_required_credentials",
  "message": "Codex cannot continue autonomously because required credentials are unavailable in this environment."
}
```

This is not `blocked_external`, not `needs_user`, and not a repair loop. It is a
clean stop projection for cases where no autonomous next action exists.

## Graph Truth

Graph truth is only:

```text
PLAN.yaml tasks + depends_on + STATE.json task statuses/supersession
```

Waves are derived. Waves are never stored as editable truth.

`PLAN.yaml` owns:

```yaml
tasks:
  - id: T1
    title: Build status projection
    depends_on: []
    write_paths:
      - crates/codex1/src/status/**
    proof:
      - cargo test -p codex1 status_contract
  - id: T2
    title: Build Ralph adapter
    depends_on: [T1]
```

`STATE.json` owns whether `T1` or `T2` is pending, in progress, complete,
superseded, or cancelled.

## DAG Validation

`codex1 plan check` should validate:

- Unique task IDs.
- No missing dependencies.
- No duplicate dependencies.
- No dependency cycles.
- No dependency on cancelled tasks unless explicitly allowed by supersession.
- Required proof commands are present for tasks that need proof.
- Required review boundaries are declared for graph/large/risky work.

Cycle errors should be exact:

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "PLAN_GRAPH_CYCLE",
  "cycle": ["T2", "T5", "T2"]
}
```

## Wave Derivation

Derive waves every time:

```text
load PLAN.yaml tasks
exclude superseded/cancelled tasks
validate graph
mark a dependency satisfied when its current status is complete or review_clean
find all pending tasks whose dependencies are satisfied
compute topological level for each ready task
current wave = ready tasks at the lowest topological level
wave_id = "W" + (level + 1)
```

Dependency-satisfying statuses are exactly:

```text
complete
review_clean
```

`proof_submitted` is not dependency-satisfying. `review_pending` is not
dependency-satisfying. `superseded` dependencies are handled by the replan or
supersession rules for that task, not silently treated as success.

Topological level:

```text
level(task) = 0 if no dependencies
level(task) = 1 + max(level(dependency)) otherwise
```

If `T2` and `T3` both depend on `T1`, and `T1` is complete, then `T2` and `T3`
are in `W2`.

## Parallel Safety

A ready wave is parallel-safe only when all ready tasks have:

- No overlapping `write_paths`.
- No overlapping `exclusive_resources`.
- `unknown_side_effects: false`.
- No conflicting external service assumptions.
- No unsafe shared mutation such as package manager changes, migrations, schema
  changes, or global config changes unless explicitly isolated.
- No task that reviews another task in the same wave.

If not parallel-safe, `codex1 task next --json` should return one safest next
task plus `parallel_blockers`.

Example:

```json
{
  "wave_id": "W2",
  "tasks": ["T2", "T3"],
  "parallel_safe": false,
  "recommended_task": "T2",
  "parallel_blockers": [
    {
      "tasks": ["T2", "T3"],
      "reason": "overlapping_write_paths",
      "paths": ["crates/codex1/src/status/**"]
    }
  ]
}
```

## Close Readiness

`codex1 status --json` and `codex1 close check --json` must call the same
readiness function.

If status says close is ready and close check says it is not, the design has
split truth.

Close readiness has two phases.

`requires_mission_close_review` is true for graph, large, risky, or explicitly
configured missions. It is false for the first-slice simple normal mission.

Pre-close readiness requires:

- Outcome ratified.
- Plan locked.
- No pending required steps/tasks.
- No current accepted blocking findings.
- No replan required.
- Required current reviews are clean.

If pre-close readiness is satisfied and the mission does not require
mission-close review, status/close-check should project close state as
`close_complete_ready` and return next action `close_complete`.

If pre-close readiness is satisfied, mission-close review is required, and
mission-close review has not passed, status/close-check should project close
state as `ready_for_mission_close_review` or `mission_close_review_open` and
return next action `close_review`.

If required mission-close review has passed, status/close-check should project
close state as `mission_close_review_passed` or `close_complete_ready` and
return next action `close_complete`.

Status and close check are read-only projections. They must not mutate close
state. The mutating commands are:

- `codex1 close record-review --json`: records mission-close review outcome and
  may transition close state from `mission_close_review_open` to
  `mission_close_review_passed`.
- `codex1 close complete --json`: writes/verifies `CLOSEOUT.md` and records
  terminal state.

`codex1 close check --json` checks pre-close readiness only. A missing
`CLOSEOUT.md` does not make pre-close readiness fail; it makes the next action
`close_complete`.

`codex1 close complete --json` writes or verifies `CLOSEOUT.md`, then records
terminal state. When terminal state is recorded, `CLOSEOUT.md` must exist and
match the current revision.

## Overengineering Guards

Do not build:

- Event replay authority in v1.
- Stored waves.
- A workflow daemon.
- Subagent identity gates.
- `blocked_external` as normal mission state.
- `validation_required` as top-level verdict.
- Separate Ralph state.
- Multiple status projections that can disagree.
- Normal mode secretly shaped like graph mode.

The smallest solid substrate is:

```text
locked outcome
locked plan
atomic state revision
append-only audit event
derived graph/wave projection
single status JSON
Ralph as a thin adapter over status.stop
```
