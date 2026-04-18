# Codex1 V2 PRD

Status: draft for Claude Code build planning
Date: 2026-04-18

## Warning

Do not build V2 by extending the current Codex1 mission/Ralph machinery.

The current repo is useful reference material, but the V1 runtime became too
large, too stateful, and too easy to perturb. V2 should preserve the high-level
product idea while rebuilding around a smaller contract:

```text
skills-first UX + small CLI contract kernel + tiny Ralph status guard +
normal bounded subagents + visible files
```

Claude Code should treat this PRD and the companion CLI contract as the source
of truth:

- `docs/codex1-v2-architecture-brief.md`
- `docs/codex1-v2-cli-contract.md`
- `docs/codex1-v2-prd.md`

## North Star

Codex1 exists so a user can enter native Codex, say what they want built,
answer the necessary clarification questions, and then let Codex carry the
mission through deep planning, bounded execution, review, repair, replan, and
mission close until the work is actually done or honestly waiting on the user.

V2 must make this feel simple:

```text
$clarify -> $plan -> $execute
```

or:

```text
$autopilot
```

The user should not feel like they are operating a giant state machine.

## Product Promise

Codex1 V2 should let a user:

1. Invoke `$clarify` and get a ratified outcome lock.
2. Invoke `$plan light|medium|hard` and get a real task DAG.
3. Invoke `$execute` and have Codex execute dependency-safe waves.
4. Have review run through bounded reviewer agents, not parent self-review.
5. Repair findings or replan when review proves the route is wrong.
6. Pause the parent loop with `$close` and talk without Ralph fighting them.
7. Invoke `$autopilot` to compose clarify, plan, execute, review, repair,
   replan, pause handling, and mission close.
8. Reach terminal completion only after mission-close review passes.

## Core Architecture

V2 has five layers:

| Layer | Responsibility |
| --- | --- |
| Public skills | User-facing workflow, interviewing, planning posture, orchestration instructions |
| CLI contract kernel | Deterministic validation, status projection, DAG checks, wave derivation, task/review/close state |
| Ralph hook | Calls `codex1 status --mission <id> --json` and blocks only active parent loops with required next action |
| Subagents | Normal bounded worker/reviewer/advisor lanes, never inside Ralph |
| Visible files | Outcome lock, blueprint DAG, state, events, specs, proofs, review evidence |

The CLI is the deterministic backend. The skills are the product surface.

## Non-Negotiable Design Rules

- No hidden runtime daemon.
- No external wrapper that controls Codex outside native Codex.
- No giant hidden state machine.
- No markdown-only machine truth.
- No multiple competing state truth surfaces.
- No stored waves as canonical truth; waves are derived.
- No parent self-review.
- No reviewer agent gate writeback.
- No subagent inside Ralph.
- No mission completion from execution alone.
- No stale child/reviewer output counted as current evidence.
- No implicit active mission resolution in V2 build; commands require
  `--mission` except `codex1 init`.

## Public Skills

V2 keeps these skills as public UX:

- `$clarify`
- `$plan`
- `$execute`
- `$review-loop`
- `$autopilot`
- `$close`

The skills should be concise and practical. They should teach Codex when and
how to call the CLI. They should not duplicate gate math or hidden runtime
logic.

### `$clarify`

Purpose: produce `OUTCOME-LOCK.md`.

Manual `$clarify` stops after the outcome lock is ready. It does not
automatically plan. `$autopilot` may continue from clarify into plan.

### `$plan`

Purpose: produce `PROGRAM-BLUEPRINT.md` with a mandatory task DAG and task
specs.

Accepted levels:

- `light`
- `medium`
- `hard`

Requested level may be escalated by risk:

```yaml
requested_level: light
risk_floor: hard
effective_level: hard
```

Every plan must include task IDs and dependencies:

```yaml
tasks:
  - id: T1
    title: Define CLI contract
    depends_on: []

  - id: T2
    title: Implement status command
    depends_on: [T1]
```

A plan without a DAG is invalid.

### `$execute`

Purpose: ask the CLI what task or wave is next, execute it, record proof, and
route to review when owed.

Execution should use:

```bash
codex1 status --mission <id> --json
codex1 task next --mission <id> --json
codex1 task start --mission <id> T1 --json
codex1 task finish --mission <id> T1 --proof specs/T1/PROOF.md --json
```

### `$review-loop`

Purpose: parent-owned review orchestration.

Reviewers produce durable reviewer outputs. Parent integrates them. Reviewers
do not invoke review-loop, clear gates, mark tasks clean, or decide mission
completion.

### `$autopilot`

Purpose: compose clarify, plan, execute, review-loop, repair/replan, and
mission close until done or honestly waiting.

Autopilot should be composition over the small commands, not a separate hidden
runtime.

### `$close`

Purpose: pause the active parent loop so the user can talk, redirect, inspect,
or stop.

`$close` is not terminal mission close. Terminal mission close is represented
by the CLI command group `codex1 mission-close`.

## CLI Contract Kernel

The `codex1` CLI should be agent-friendly:

- `--help` everywhere
- `--json` everywhere
- stable schemas
- predictable error codes
- small default output
- file output for large payloads
- deterministic ordering
- explicit `--mission`
- `--dry-run` for mutating commands
- revision guards for mutating commands

V2 command groups:

```bash
codex1 init --mission <id> --title <title> --json
codex1 validate --mission <id> --json
codex1 status --mission <id> --json

codex1 plan check --mission <id> --json
codex1 plan waves --mission <id> --json

codex1 task next --mission <id> --json
codex1 task start --mission <id> T1 --json
codex1 task finish --mission <id> T1 --proof specs/T1/PROOF.md --json

codex1 review open --mission <id> --task T1 --profiles code_bug_correctness,local_spec_intent --json
codex1 review submit --mission <id> --bundle B1 --input reviewer-output.json --json
codex1 review status --mission <id> --bundle B1 --json
codex1 review close --mission <id> --bundle B1 --json

codex1 replan record --mission <id> --reason <code> --json
codex1 replan check --mission <id> --json

codex1 mission-close check --mission <id> --json
codex1 mission-close complete --mission <id> --json
```

## Status Contract

`codex1 status --mission <id> --json` is the primary command.

Ralph should call only this command.

Example:

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

`verdict` is primary. Other fields must be internally consistent.

## Canonical Files

V2 file layout:

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

Truth ownership:

- `OUTCOME-LOCK.md` owns destination truth.
- `PROGRAM-BLUEPRINT.md` owns immutable route/DAG truth.
- `STATE.json` owns operational task/loop state.
- `events.jsonl` is audit-only unless replay semantics are specified.
- review JSON owns review evidence.

## Task DAG Requirements

Task IDs:

- must match `^T[0-9]+$`
- must never be reused within a mission
- replans append new task IDs and may supersede old ones

Task DAG fields:

```yaml
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
  unknown_side_effects: false
  proof:
    - cargo test -p codex1
  review_profiles:
    - code_bug_correctness
    - local_spec_intent
```

Mutable task status belongs in `STATE.json`, not the blueprint.

## Wave Derivation

Waves are derived from the DAG and state.

Parallel wave safety requires:

- dependency independence
- write path disjointness
- no write/read conflicts
- exclusive resource disjointness
- no unknown global side effects
- isolated worktrees/branches or patch-only worker outputs for parallel writes

Unknown side effects force serial execution.

## Task Lifecycle

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

Typical flow:

```text
planned -> ready -> in_progress -> proof_submitted -> review_owed
review_owed -> review_clean -> complete
review_owed -> review_failed -> needs_repair
needs_repair -> in_progress
needs_repair -> replan_required
```

Repair eligibility must not deadlock. `needs_repair` is allowed when a current
failed review boundary is assigned for repair and no replan-required
contradiction is open.

## Review Requirements

Review bundles must specify requirements, not just profiles:

```json
{
  "requirements": [
    {
      "id": "B1-code",
      "profile": "code_bug_correctness",
      "min_outputs": 1,
      "allowed_roles": ["reviewer"]
    }
  ]
}
```

Reviewer outputs must bind to current truth:

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

Outputs with stale graph/state/evidence bindings must be rejected or
quarantined as `STALE_OUTPUT`.

## Replan Requirements

Replan triggers:

- write scope expansion
- missing or false dependency
- interface contract change
- impossible or proxy proof row
- review contract change
- outcome meaning change
- six consecutive non-clean review loops
- invalid parallel safety assumption

Replan must preserve history. It may supersede tasks, but it must not erase
failed task truth.

## Ralph Requirements

Ralph must:

1. Run `codex1 status --mission <id> --json`.
2. Block only if `stop_policy.allow_stop == false`.
3. Use `next_action.display_message` as the stop feedback.
4. Allow stop for discussion, paused loops, subagents, reviewers, terminal
   completion, and invalid states that need user intervention.

Ralph must not inspect mission files directly.

## Build Waves

V2 should be implemented as one full product mission, but built in dependency
waves:

1. Kernel and DAG: init, validate, status, plan check, plan waves, task next.
2. Task execution: start, finish, proof refs, task run IDs, state revisions.
3. Review and repair/replan: review open/submit/status/close, replan.
4. Ralph and skills: `$execute`, `$review-loop`, `$close`.
5. Full autonomy: `$autopilot`, advisor checkpoints, mission-close review,
   end-to-end qualification.

This is not MVP scope reduction. It is dependency ordering.

## Acceptance Bar

V2 is acceptable when:

- a user can clarify a mission and get an outcome lock
- planning produces a valid DAG
- waves are derived and parallel-safe
- execution uses task start/finish and proof refs
- review uses durable reviewer outputs
- repair and replan work without stale artifacts
- Ralph only uses `codex1 status`
- `$close` pauses the parent loop without terminalizing
- `$autopilot` composes the full flow
- mission close requires clean mission-close review
- stale subagent/reviewer output cannot contaminate current truth
- every claimed status is explainable from visible files plus deterministic CLI
  output

