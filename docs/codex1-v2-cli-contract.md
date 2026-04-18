# Codex1 V2 CLI Contract

Status: draft for critique
Date: 2026-04-18

## Contract Goals

The `codex1` CLI is the deterministic contract kernel for Codex1 V2.

It should be:

- composable by Codex
- boring for humans to inspect
- strict about structure
- stable in JSON output
- conservative about safety
- incapable of hidden orchestration

The CLI should not replace native Codex skills. It should give those skills a
precise command surface.

## Agent-Friendly CLI Rules

Every command should support:

```bash
--help
--json
--mission <mission-id>
--repo-root <path>
```

For V2, every command except `codex1 init` must require `--mission`.
Implicit active mission resolution is intentionally out of scope because it
reintroduces hidden state and ambiguity. A later convenience command may add
`codex1 mission current --json`.

Commands that may mutate state should support:

```bash
--dry-run
```

Default output should be small. Large output should be written to a file and
reported by path.

Errors must be predictable JSON when `--json` is set:

```json
{
  "ok": false,
  "schema": "codex1.error.v1",
  "code": "TASK_BLOCKED",
  "message": "Task T3 cannot start because T2 has not passed review.",
  "retryable": false,
  "exit_code": 2,
  "hint": "Run codex1 task status --mission example T2 --json.",
  "details": {
    "task_id": "T3",
    "blocked_by": ["T2"]
  }
}
```

Success output should always include:

```json
{
  "ok": true,
  "schema": "codex1.<command>.v1"
}
```

Lists in JSON output must be deterministically sorted unless order is
semantically meaningful.

## Mission File Layout

```text
PLANS/<mission-id>/
  OUTCOME-LOCK.md
  PROGRAM-BLUEPRINT.md
  STATE.json
  events.jsonl
  specs/
    T1/
      SPEC.md
      PROOF.md
      REVIEW.md
  reviews/
    B1.json
    outputs/
      R1.json
```

## Command Groups

The final V2 surface includes all command groups below. Implementation should
still happen in dependency waves: status/DAG first, then task execution, then
review/replan, then Ralph/skills/autopilot/mission-close composition.

### Mission

```bash
codex1 init --mission <id> --title <title> --json
codex1 status --mission <id> --json
codex1 validate --mission <id> --json
```

### Lock

```bash
codex1 lock check --mission <id> --json
```

### Plan

```bash
codex1 plan check --mission <id> --json
codex1 plan graph --mission <id> --json
codex1 plan waves --mission <id> --json
```

### Task

```bash
codex1 task next --mission <id> --json
codex1 task start --mission <id> T1 --json
codex1 task finish --mission <id> T1 --proof specs/T1/PROOF.md --json
codex1 task status --mission <id> T1 --json
```

### Review

```bash
codex1 review open --mission <id> --task T1 --profiles code_bug_correctness,local_spec_intent --json
codex1 review submit --mission <id> --bundle B1 --input reviewer-output.json --json
codex1 review status --mission <id> --bundle B1 --json
codex1 review close --mission <id> --bundle B1 --json
```

### Replan

```bash
codex1 replan record --mission <id> --reason <code> --json
codex1 replan check --mission <id> --json
```

### Parent Loop

Backs the `$execute`, `$review-loop`, `$autopilot`, and `$close` skills.
The parent thread `activate`s a mode when it begins orchestrating and
`deactivate`s on completion. `pause` flips the loop into discussion mode
(Ralph allows stop with reason `discussion_pause`); `resume` restores
blocking stop behaviour.

```bash
codex1 parent-loop activate   --mission <id> --mode execute|review|autopilot|close --json
codex1 parent-loop deactivate --mission <id> --json
codex1 parent-loop pause      --mission <id> --json
codex1 parent-loop resume     --mission <id> --json
```

Invariants:

- `activate` with `mode: none` is rejected — use `deactivate` instead.
- `pause`/`resume` are no-ops (error) when no loop is active.
- Each command is a single atomic `state_revision` bump and appends one
  `parent_loop_{activated|deactivated|paused|resumed}` event.
- Status envelope derives `parent_loop.active` from `mode != none`;
  `stop_policy` is computed from `(active, paused, verdict)` per
  `status::derive_stop_policy` (see Ralph Contract below).

### Mission Close

```bash
codex1 mission-close check --mission <id> --json
codex1 mission-close complete --mission <id> --json
```

### Ralph

Ralph should call only:

```bash
codex1 status --mission <id> --json
```

For V2, Ralph must provide `--mission`. Mission auto-detection can be added
later only through an explicit `mission current` command.

## Status Schema

`codex1 status --mission <id> --json` is the primary contract.

```json
{
  "ok": true,
  "schema": "codex1.status.v1",
  "mission_id": "example",
  "state_revision": 42,
  "phase": "executing",
  "terminality": "non_terminal",
  "verdict": "continue_required",
  "parent_loop": {
    "active": true,
    "mode": "execute",
    "paused": false
  },
  "stop_policy": {
    "allow_stop": false,
    "reason": "active_parent_loop"
  },
  "next_action": {
    "kind": "start_task",
    "task_id": "T2",
    "args": ["--mission", "example", "T2"],
    "display_message": "Start task T2."
  },
  "ready_tasks": ["T2", "T3"],
  "running_tasks": [],
  "review_required": [],
  "blocked": [],
  "stale": [],
  "required_user_decision": null
}
```

Allowed `phase` values:

- `clarify`
- `planning`
- `executing`
- `reviewing`
- `repairing`
- `replanning`
- `waiting`
- `mission_close`
- `complete`

Allowed `terminality` values:

- `non_terminal`
- `terminal`

Allowed `verdict` values:

- `continue_required`
- `needs_user`
- `blocked`
- `complete`
- `invalid_state`

`verdict` is primary. `phase`, `terminality`, `parent_loop`, `stop_policy`,
and `next_action` are derived fields and must be internally consistent with the
verdict.

## Plan DAG Schema

`PROGRAM-BLUEPRINT.md` must contain one machine-readable task block.

V2 uses YAML as the canonical plan DAG format. The YAML block must be extracted
from `PROGRAM-BLUEPRINT.md` with strict markers:

```text
<!-- codex1:plan-dag:start -->
<yaml plan dag>
<!-- codex1:plan-dag:end -->
```

```yaml
planning:
  requested_level: medium
  risk_floor: hard
  effective_level: hard
  graph_revision: 1

tasks:
  - id: T1
    title: Define CLI contract
    kind: design
    depends_on: []
    spec_ref: specs/T1/SPEC.md
    read_paths:
      - docs/**
    write_paths:
      - docs/codex1-v2-cli-contract.md
    exclusive_resources: []
    proof:
      - CLI contract written
    review_profiles:
      - local_spec_intent

  - id: T2
    title: Implement status command
    kind: code
    depends_on: [T1]
    spec_ref: specs/T2/SPEC.md
    read_paths:
      - crates/codex1/**
    write_paths:
      - crates/codex1/**
    exclusive_resources:
      - status_schema
    proof:
      - cargo test -p codex1
    review_profiles:
      - code_bug_correctness
      - local_spec_intent
```

Task ID rules:

- must match `^T[0-9]+$`
- must never be reused within a mission
- dependencies must reference existing IDs
- graph must be acyclic
- completed task history must not be erased

## Wave Derivation

`codex1 plan waves --mission <id> --json` derives waves from the DAG and
`STATE.json`.

```json
{
  "ok": true,
  "mission_id": "example",
  "graph_revision": 1,
  "waves": [
    {
      "id": "W1",
      "tasks": ["T1"],
      "mode": "serial",
      "eligible": true,
      "safety": {
        "dependency_independent": true,
        "write_paths_disjoint": true,
        "read_write_conflicts": [],
        "exclusive_resources_disjoint": true,
        "unknown_side_effects": false
      }
    }
  ]
}
```

A task is wave-eligible when:

- it is `ready` or `needs_repair`
- if `ready`, it has no open blocking review finding
- if `needs_repair`, it has a current failed review boundary assigned for
  repair and no replan-required contradiction
- all dependencies are `review_clean` or `complete`
- its spec exists
- its plan graph revision is current

Parallel wave safety requires:

- no dependency edges inside the wave
- no pairwise write/write conflict
- no write/read conflict
- disjoint exclusive resources
- no unknown global side effects

## State Schema

`STATE.json` should be compact.

```json
{
  "mission_id": "example",
  "state_revision": 42,
  "phase": "executing",
  "parent_loop": {
    "mode": "execute",
    "paused": false
  },
  "tasks": {
    "T1": {
      "status": "review_clean",
      "started_at": "2026-04-18T10:00:00Z",
      "finished_at": "2026-04-18T10:15:00Z",
      "reviewed_at": "2026-04-18T10:30:00Z"
    },
    "T2": {
      "status": "ready"
    }
  }
}
```

Allowed task statuses:

- `planned`
- `ready`
- `in_progress`
- `proof_submitted`
- `review_owed`
- `review_failed`
- `needs_repair`
- `replan_required`
- `review_clean`
- `complete`
- `superseded`

## Events

All state mutations update `STATE.json` atomically and append to `events.jsonl`
for audit.

Example:

```json
{"seq":1,"kind":"task_started","task_id":"T1","at":"2026-04-18T10:00:00Z"}
{"seq":2,"kind":"task_finished","task_id":"T1","proof_ref":"specs/T1/PROOF.md","at":"2026-04-18T10:15:00Z"}
{"seq":3,"kind":"review_opened","task_id":"T1","bundle_id":"B1","at":"2026-04-18T10:16:00Z"}
```

`STATE.json` is authoritative operational state for V2. `events.jsonl` is
audit-only unless replay semantics are specified and tested.

Every mutating command must check the current `state_revision` and write
atomically under a lock. Mutating commands should accept `--expect-revision`;
stale callers receive `REVISION_CONFLICT`.

## Review Bundle Schema

```json
{
  "bundle_id": "B1",
  "mission_id": "example",
  "graph_revision": 1,
  "state_revision": 42,
  "target": {
    "kind": "task",
    "task_id": "T1"
  },
  "requirements": [
    {
      "id": "B1-code",
      "profile": "code_bug_correctness",
      "min_outputs": 1,
      "allowed_roles": ["reviewer"]
    },
    {
      "id": "B1-intent",
      "profile": "local_spec_intent",
      "min_outputs": 1,
      "allowed_roles": ["reviewer"]
    }
  ],
  "evidence_refs": [
    "PLANS/example/specs/T1/SPEC.md",
    "PLANS/example/specs/T1/PROOF.md"
  ],
  "status": "open"
}
```

## Reviewer Output Schema

Clean output:

```json
{
  "bundle_id": "B1",
  "reviewer_id": "R1",
  "requirement_id": "B1-code",
  "profile": "code_bug_correctness",
  "graph_revision": 1,
  "state_revision": 42,
  "evidence_snapshot_hash": "sha256:...",
  "result": "none",
  "findings": []
}
```

Findings output:

```json
{
  "bundle_id": "B1",
  "reviewer_id": "R2",
  "requirement_id": "B1-intent",
  "profile": "local_spec_intent",
  "graph_revision": 1,
  "state_revision": 42,
  "evidence_snapshot_hash": "sha256:...",
  "result": "findings",
  "findings": [
    {
      "severity": "P1",
      "title": "Proof does not exercise the claimed behavior",
      "evidence_refs": ["PLANS/example/specs/T1/PROOF.md:12"],
      "rationale": "The proof checks command help but not the task behavior.",
      "suggested_next_action": "Add a behavior test and rerun review."
    }
  ]
}
```

Review status:

```json
{
  "ok": true,
  "bundle_id": "B1",
  "clean": false,
  "missing_profiles": [],
  "blocking_findings": 1,
  "next_action": {
    "kind": "repair_task",
    "task_id": "T1"
  }
}
```

## Mission Close Check

`codex1 mission-close check --mission <id> --json` passes only when:

- all non-superseded required tasks are `review_clean` or `complete`
- no P0/P1/P2 findings are open
- all proof rows are represented
- required integration reviews are clean
- mission-close review is clean
- `STATE.json` is valid and current

Example:

```json
{
  "ok": true,
  "can_close": true,
  "mission_id": "example"
}
```

Blocked close:

```json
{
  "ok": true,
  "can_close": false,
  "blocking_reasons": [
    {
      "code": "TASK_REVIEW_OWED",
      "task_id": "T2"
    }
  ]
}
```

## Ralph Contract

Ralph must consume only `codex1 status --mission <id> --json`.

Block:

```json
{
  "ok": true,
  "stop_policy": {
    "allow_stop": false,
    "reason": "active_parent_loop"
  },
  "parent_loop": {
    "active": true
  },
  "verdict": "continue_required",
  "next_action": {
    "kind": "start_task",
    "display_message": "Continue task T2."
  }
}
```

Allow:

```json
{
  "ok": true,
  "stop_policy": {
    "allow_stop": true,
    "reason": "complete"
  },
  "parent_loop": {
    "active": false
  },
  "verdict": "complete"
}
```

Ralph should not inspect events, reviews, specs, or gates itself.

## Build Sequencing

V2 should be built as one full product mission, not a reduced MVP. The build
order should still follow the dependency graph:

1. Kernel and DAG: `init`, `validate`, `status`, `plan check`, `plan waves`,
   `task next`.
2. Task execution: `task start`, `task finish`, proof refs, task run IDs, state
   revisions.
3. Review and repair/replan: `review open`, `review submit`, `review status`,
   `review close`, `replan record/check`.
4. Ralph and skill composition: Ralph calls only `codex1 status --mission <id>
   --json`; `$execute`, `$review-loop`, and `$close` compose the CLI.
5. Full autonomy and close: `$autopilot`, advisor checkpoints, mission-close
   check/complete, end-to-end qualification.

Do not build autopilot first. Autopilot should be an emergent composition of
the smaller commands, but the final V2 product target includes autopilot.
