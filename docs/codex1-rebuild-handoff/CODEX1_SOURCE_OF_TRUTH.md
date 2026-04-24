# Codex1 Source Of Truth

This file is the implementation authority for the next Codex1 build.

Every implementation agent and every subagent must read this file before
writing code. If another document, prompt, review comment, or memory disagrees
with this file, this file wins until the user edits it.

Codex1 is one product. Graph mode, planned review boundaries, replan, and
mission-close review are not a separate product and are not deferred out of the
product architecture. The implementation may be staged internally, but the code
must be designed around the whole product from day one.

Do not copy source code from any failed implementation attempt. Use prior review
findings only as lessons and regression tests.

## Product Thesis

Codex1 is a skills-first durable workflow substrate for native Codex.

Codex stays the actor. Codex1 supplies deterministic rails:

```text
skills-first UX
codex1 CLI
visible durable mission files under PLANS/<mission-id>/
canonical status projection
Ralph Stop-hook guard over status
normal and graph planning
planned review boundaries
repair and replan loops
mission-close review
stable JSON for every machine path
```

Codex1 is not an agent runtime. It does not replace Codex turns, tools,
subagents, or review protocol. It gives Codex a durable state machine and a
single integration artifact:

```bash
codex1 status --json
```

Everything else exists to make that projection truthful.

## Lessons From Failed Builds

The failure pattern was not one bug. It was implementing command breadth before
invariant depth.

The recurring review findings were:

- command names existed before their safety contracts existed
- mutable state was trusted when readiness should have been derived
- artifact writes escaped transactions
- state could commit while command output reported failure
- event audit drift was hidden
- locked plans were mutable in practice
- graph planning existed without graph validation and scheduling invariants
- review records could pass without complete triage
- stale proofs and closeouts could complete work
- path validation was lexical instead of resolved and symlink-safe
- Ralph could block from unsafe or malformed status projections
- JSON mode did not always return JSON
- active mission convenience leaked into mutating writes

The correct implementation order is therefore:

```text
schemas
path security
artifact parsers
canonical digests
state validation
transaction engine
plan and graph validators
status and stop projectors
close readiness engine
review and replan validators
command adapters
doctor and packaging
```

Commands are adapters. They are not the product brain.

## Non-Negotiables

1. `codex1 status --json` is the central integration artifact.
2. Status is derived from current artifacts plus state. It does not blindly trust
   mutable readiness fields.
3. Every successful mutation increments `STATE.json.revision` exactly once.
4. Every successful mutation appends exactly one event. If an event append fails
   after the state commit, the command returns success with an explicit
   audit-drift warning. It must not crash after committing state.
5. Every mutation that writes artifacts must hold the mission lock before
   writing both artifacts and state.
6. Ordinary mutating commands require explicit `--mission <mission-id>`.
   `ACTIVE.json` is allowed for read defaults, Ralph, and explicit selection or
   activation commands only.
7. Mission IDs are IDs, not paths. They cannot contain slash, backslash, dot-dot,
   absolute-path syntax, NUL, or empty segments.
8. Resolved mission roots must remain under the resolved `PLANS/` root. Symlink
   escapes are rejected.
9. `OUTCOME.md` and `PLAN.yaml` digests are recomputed on every status read.
   Stale ratified or locked artifacts project `invalid_state` and Ralph
   fail-opens.
10. A locked plan cannot be changed by plain `plan lock`. Relock requires the
    explicit replan path.
11. Close readiness is derived from locked plan work, state, review boundaries,
    replan state, and close review gates. It never trusts `close.state` alone.
12. Ralph reads status only. Ralph blocks only from a safe blockable status
    projection.
13. Graph mode must have real graph validation, dependency satisfaction, and
    wave safety before graph plans can lock.
14. Review findings are observations until fully triaged. Partial triage cannot
    pass a boundary.
15. Proof and closeout evidence must be anchored to the current mission,
    revision, outcome digest, and plan digest.
16. All `--json` command outputs, including argument errors, are stable JSON.
17. The implementation must be modular. No monolithic file that hides the state
    machine in thousands of lines.
18. Tests must encode every invariant here.

## Durable File Layout

The repository root contains a `PLANS/` directory when Codex1 durable state is
used.

```text
PLANS/
  ACTIVE.json
  <mission-id>/
    OUTCOME.md
    PLAN.yaml
    STATE.json
    EVENTS.jsonl
    CLOSEOUT.md
    proofs/
      <subject-id>.<proof-id>.json
    reviews/
      <review-id>.json
```

Only `PLANS/<mission-id>/` is a mission root. Mission IDs are slug-like stable
IDs:

```text
allowed:   codex1-rebuild, demo_001, mission-42
rejected:  ../demo, /tmp/demo, demo/other, demo\other, ., .., ""
```

Implement a single mission resolver module. No command should build mission
paths by hand.

## Path Security

All path security checks are resolved-path checks, not lexical checks.

Required behavior:

```text
repo_root = canonicalized repository root
plans_root = canonicalized repo_root/PLANS
mission_root_candidate = repo_root/PLANS/<mission-id>
mission_root_resolved = canonicalized existing mission root
mission_root_resolved must be equal to or inside plans_root
STATE.json.mission_id must equal selected mission id
```

If the mission directory does not exist yet, validate the path lexically and
validate the parent `PLANS/` root. After creation, canonicalize it.

Reject:

- symlinked mission directories resolving outside `PLANS/`
- symlinked `STATE.json`, `PLAN.yaml`, `OUTCOME.md`, `EVENTS.jsonl`, or
  `CLOSEOUT.md` resolving outside the mission root
- closeout paths other than `CLOSEOUT.md`
- proof/review paths outside `proofs/` and `reviews/`
- `ACTIVE.json` mission IDs that fail mission ID validation
- non-object `ACTIVE.json`; ignore it with a warning rather than crashing

## JSON Error Envelope

Every command with `--json` must return one of:

```json
{
  "schema_version": "codex1.result.v1",
  "ok": true,
  "data": {},
  "warnings": []
}
```

or:

```json
{
  "schema_version": "codex1.error.v1",
  "ok": false,
  "error": {
    "code": "ARGUMENT_ERROR",
    "message": "human-readable summary",
    "retryable": false,
    "details": {}
  }
}
```

Argument parsing must support JSON errors. Do not let the CLI parser emit plain
text when `--json` is present. If the parser library cannot do this directly,
pre-scan for `--json` and convert parser errors.

Canonical error codes include:

```text
ARGUMENT_ERROR
MISSION_NOT_FOUND
MISSION_ID_INVALID
MISSION_PATH_ESCAPE
STATE_INVALID
ARTIFACT_INVALID
DIGEST_MISMATCH
REVISION_CONFLICT
MISSION_LOCKED
PLAN_ALREADY_LOCKED
PLAN_NOT_LOCKED
PLAN_INVALID
OUTCOME_INVALID
LOOP_NOT_ACTIVATABLE
TASK_NOT_READY
TASK_NOT_IN_PROGRESS
PROOF_INVALID
REVIEW_TRIAGE_REQUIRED
REVIEW_INVALID
REPLAN_REQUIRED
CLOSE_NOT_READY
INTERNAL_ERROR
```

`REVISION_CONFLICT` must include:

```json
{
  "current_revision": 17,
  "expected_revision": 16,
  "retryable": true
}
```

## Artifact Parsing And Digests

Use structured parsing. Do not parse required machine fields from prose.

### Canonical Digest

Canonical digest is:

```text
sha256:<hex of canonical JSON bytes>
```

Canonical JSON:

- parsed structured object only
- sorted object keys
- UTF-8
- no insignificant whitespace
- preserves array order
- excludes purely decorative markdown prose unless explicitly stated

Whitespace-only changes and YAML key order changes must not change the digest.
Semantic field changes must change the digest.

### OUTCOME.md

`OUTCOME.md` is YAML frontmatter plus optional human markdown. Only YAML
frontmatter is machine parsed and digested.

Required frontmatter:

```yaml
schema_version: codex1.outcome.v1
mission_id: demo
title: Short mission title
original_user_goal: User's original request in durable form
destination: What must be true when done
acceptance_criteria:
  - Concrete pass/fail criterion
constraints:
  - Constraint or explicit "none beyond repository and user instructions"
non_goals:
  - Explicit non-goal or explicit "none"
proof_expectations:
  - Required proof or validation expectation
pr_intent:
  open_pr: false
  base_branch: null
  title: null
```

Optional frontmatter:

```yaml
definitions: {}
quality_bar: []
review_expectations: []
known_risks: []
user_only_actions: []
resolved_questions: []
```

Validation:

- required scalar strings must be non-empty and not placeholders
- required arrays must be non-empty
- array items must be non-empty and not placeholders
- `constraints` and `non_goals` are required non-empty arrays; an explicit
  "none" item is acceptable
- `mission_id` must match selected mission
- no `status` field is required in the artifact; ratification truth lives in
  `STATE.json`

### PLAN.yaml

`PLAN.yaml` is a YAML object. It is the locked route once locked.

Common required fields:

```yaml
schema_version: codex1.plan.v1
mission_id: demo
mode: normal # normal | graph
level: medium # small | medium | large | risky
title: Plan title
acceptance:
  - Mirrors or refines outcome acceptance
validation:
  - Command/check expectation
requires_mission_close_review: false
```

Normal mode:

```yaml
steps:
  - id: S1
    title: Implement one thing
    acceptance:
      - Step-level criterion
    proof_expectations:
      - Test or inspection expectation
```

Graph mode:

```yaml
tasks:
  - id: T1
    title: Implement shared parser
    depends_on: []
    write_paths:
      - src/parser.rs
    exclusive_resources:
      - parser-core
    parallel_safe: true
    unknown_side_effects: false
    acceptance:
      - Criterion
    proof_expectations:
      - Test or inspection expectation
    review_boundary: RB-T1
review_boundaries:
  - id: RB-T1
    after_tasks:
      - T1
    required: true
    max_repair_rounds: 2
```

Plan validation:

- schema version exact
- mission ID matches
- mode and level recognized
- normal plan has non-empty unique step IDs and no graph-only fields
- graph plan has non-empty unique task IDs
- dependencies point to existing task IDs
- dependencies contain no duplicates
- graph has no cycles
- task IDs and boundary IDs are unique within their namespaces
- review boundary `after_tasks` references existing task IDs
- every task with a `review_boundary` references an existing boundary
- `write_paths` are relative, non-empty, and do not escape the repository
- `exclusive_resources` are strings if present
- `parallel_safe` and `unknown_side_effects` are booleans
- required strings and arrays are not placeholders

Graph plans cannot lock if graph validation is incomplete.

## STATE.json Schema

`STATE.json` is mutable mission truth.

It must be a JSON object. Non-object JSON is invalid state, not a traceback.

Minimum shape:

```json
{
  "schema_version": "codex1.state.v1",
  "mission_id": "demo",
  "revision": 0,
  "created_at": "2026-04-24T00:00:00Z",
  "updated_at": "2026-04-24T00:00:00Z",
  "outcome": {
    "ratified": false,
    "ratified_revision": null,
    "outcome_digest": null
  },
  "plan": {
    "mode": null,
    "level": null,
    "locked": false,
    "locked_revision": null,
    "plan_digest": null,
    "outcome_digest_at_lock": null
  },
  "loop": {
    "active": false,
    "paused": false,
    "mode": "none"
  },
  "steps": {},
  "tasks": {},
  "active_wave": null,
  "reviews": {},
  "replan": {
    "required": false,
    "reason": null,
    "supersedes": []
  },
  "close": {
    "requires_mission_close_review": false,
    "mission_close_review": {
      "state": "not_required",
      "review_id": null
    },
    "closeout_digest": null
  },
  "terminal": {
    "complete": false,
    "completed_revision": null,
    "completed_at": null
  }
}
```

Validation:

- `revision` is an integer >= 0
- booleans must be booleans, not truthy strings
- `loop.mode` is one of `none`, `execute`, `autopilot`, `review_loop`
- all maps are objects
- every state step/task key equals its contained ID
- state mission ID equals selected mission ID
- invalid state projects `invalid_state` and Ralph allows stop

Task/step statuses:

```text
pending
in_progress
complete
superseded
cancelled
```

Keep review cleanliness in review boundary state, not task state.

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

## Events

`EVENTS.jsonl` is audit, not replay authority.

Each event is one JSON object line:

```json
{
  "schema_version": "codex1.event.v1",
  "event_id": "evt_...",
  "mission_id": "demo",
  "kind": "outcome_ratified",
  "revision_before": 0,
  "revision_after": 1,
  "created_at": "2026-04-24T00:00:00Z",
  "details": {}
}
```

Rules:

- every successful mutation appends exactly one event
- `revision_after == revision_before + 1`
- missing `EVENTS.jsonl` with `STATE.json.revision > 0` is audit drift
- malformed JSON lines are drift warnings
- JSON values that are not objects are malformed event rows
- object rows missing integer `revision_after` are malformed event rows
- event append failure after a committed state returns success with warning:

```json
{
  "code": "EVENT_APPEND_FAILED",
  "message": "STATE.json was committed, but EVENTS.jsonl append failed; doctor will report audit drift."
}
```

Do not crash after committing state.

## Transaction Protocol

All mutating commands must go through one transaction helper.

Required protocol:

```text
1. resolve explicit mission id unless command is init/select/activate-style
2. validate mission path and selected STATE mission id
3. acquire mission-local lock
4. re-read STATE.json and current artifacts under lock
5. validate revision and optional --expect-revision
6. validate current digests if command depends on ratified/locked artifacts
7. perform all artifact writes under lock
8. produce next STATE object
9. atomic write STATE.json with temp file + fsync + rename
10. append one event
11. if append fails after state write, return ok with audit warning
12. release lock
```

Artifact writes must not happen before the lock. The failed-branch bug was
`plan scaffold` overwriting `PLAN.yaml` even when the mission lock failed.

Lock behavior:

- lock file: `.codex1.lock` inside mission root
- use OS advisory locking or atomic lock creation
- stale-lock handling must be deterministic and tested
- never leave partial JSON files

## Command Surface

The CLI must expose the product surface. Commands can be implemented in stages,
but a command must not advertise success until it performs the real transition.

Foundation commands:

```text
codex1 init --mission <id> --json
codex1 status --json [--repo-root <path>] [--mission <id>]
codex1 doctor --json [--repo-root <path>] [--mission <id>] [--e2e]

codex1 outcome check --mission <id> --json
codex1 outcome ratify --mission <id> --json --expect-revision <n>

codex1 plan choose-mode --mission <id> --mode normal|graph --json
codex1 plan choose-level --mission <id> --level small|medium|large|risky --json
codex1 plan scaffold --mission <id> --mode normal|graph --level <level> --json
codex1 plan check --mission <id> --json
codex1 plan lock --mission <id> --json --expect-revision <n>
codex1 plan lock --mission <id> --json --replan --expect-revision <n>

codex1 task next --mission <id> --json
codex1 task start <id> --mission <id> --json --expect-revision <n>
codex1 task finish <id> --mission <id> --proof <path> --json --expect-revision <n>

codex1 review start <boundary-id> --mission <id> --json --expect-revision <n>
codex1 review record <boundary-id> --mission <id> --raw-file <path> --adjudication-file <path> --json --expect-revision <n>
codex1 review repair-record <boundary-id> --mission <id> --proof <path> --json --expect-revision <n>

codex1 replan check --mission <id> --json
codex1 replan record --mission <id> --reason <text> --json --expect-revision <n>

codex1 loop activate --mission <id> --mode execute|autopilot|review_loop --json --expect-revision <n>
codex1 loop pause --mission <id> --json --expect-revision <n>
codex1 loop resume --mission <id> --json --expect-revision <n>
codex1 loop deactivate --mission <id> --json --expect-revision <n>

codex1 close check --mission <id> --json
codex1 close complete --mission <id> --json --expect-revision <n>
codex1 close record-review --mission <id> --review-id <id> --json --expect-revision <n>

codex1 ralph stop-hook --json
```

`choose-mode` and `choose-level` are real product commands. They should parse
their flags and record deterministic choices. They must not be fake stubs.

Ordinary writes require `--mission`. Read-only commands may use cwd or
`ACTIVE.json` defaults.

## Plan Lock Rules

`plan lock`:

- requires ratified outcome
- requires current `OUTCOME.md` digest equals `STATE.outcome.outcome_digest`
- validates `PLAN.yaml`
- rejects if `STATE.plan.locked == true` unless `--replan` is present
- initializes all state step/task records from the locked plan
- initializes review boundary records from the locked plan
- stores `plan_digest`
- stores `outcome_digest_at_lock`
- stores `locked_revision`

Plain `plan lock` cannot silently relock. If the user edits `PLAN.yaml` after
lock, status projects `invalid_state` with `DIGEST_MISMATCH`.

`plan lock --replan`:

- requires `STATE.replan.required == true`
- validates new `PLAN.yaml`
- must not reuse IDs for superseded task semantics unless explicitly allowed
  and tested
- updates plan digest and state route
- marks superseded tasks/boundaries as superseded
- clears `replan.required`
- appends one event

## Loop Activation

`loop activate`:

- requires ratified fresh outcome
- requires locked fresh plan
- requires terminal not complete
- sets `loop.active = true`
- sets `loop.paused = false`
- sets `loop.mode`
- writes or updates `PLANS/ACTIVE.json`

The active pointer update must not produce a false failure after committing
state. Choose one of these designs and test it:

1. write `ACTIVE.json` under the same lock before committing state; if it fails,
   do not commit state
2. commit state first, then if `ACTIVE.json` fails, return `ok: true` with an
   `ACTIVE_POINTER_WRITE_FAILED` warning

Do not return an error after `loop.active` has been committed.

Inactive loops project inactive before replan, review, or close work. Paused
loops project paused before required work. Ralph does not block inactive,
paused, or `loop.mode == "none"`.

## Status Projection

Implement one pure status projector:

```text
project_status(snapshot) -> Status
```

Every read command and Ralph consumes this projector. No command computes a
different readiness order.

Status output:

```json
{
  "schema_version": "codex1.status.v1",
  "mission_id": "demo",
  "revision": 12,
  "verdict": "continue_required",
  "phase": "execute",
  "warnings": [],
  "digests": {
    "outcome_current": "sha256:...",
    "outcome_ratified": "sha256:...",
    "plan_current": "sha256:...",
    "plan_locked": "sha256:..."
  },
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "ready_steps": [],
  "ready_tasks": [],
  "ready_wave": null,
  "review": {
    "pending_boundaries": [],
    "accepted_blocking_count": 0
  },
  "replan": {
    "required": false,
    "reason": null
  },
  "close": {
    "ready": false,
    "requires_mission_close_review": false,
    "mission_close_review_state": "not_required"
  },
  "next_action": {
    "kind": "run_step",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "step_id": "S1",
    "task_ids": []
  },
  "stop": {
    "allow": false,
    "reason": "active_required_work",
    "message": "Codex1 has active required work: run S1. Continue it, or use $interrupt / codex1 loop pause to pause intentionally. If this is wrong, explain briefly and stop; Ralph will allow the next stop."
  }
}
```

Canonical verdicts:

```text
inactive
paused
invalid_state
clarify_required
plan_required
continue_required
review_required
replan_required
close_required
complete
explain_and_stop
```

Do not invent noncanonical verdict strings.

Priority order:

```text
1. no selected mission -> inactive / none
2. terminal complete -> complete / none
3. corrupt or unsupported state -> invalid_state / explain_and_stop / stop allow
4. selected mission id mismatch -> invalid_state / explain_and_stop / stop allow
5. invalid mission paths or symlink escape -> invalid_state / explain_and_stop / stop allow
6. stale ratified outcome digest -> invalid_state / explain_and_stop / stop allow
7. stale locked plan digest -> invalid_state / explain_and_stop / stop allow
8. invalid locked plan/state mismatch -> invalid_state / explain_and_stop / stop allow
9. paused loop -> paused / none / stop allow
10. inactive loop -> inactive / none / stop allow
11. unratified outcome -> clarify_required / explain_and_stop
12. missing or unlocked plan -> plan_required / explain_and_stop
13. replan.required -> replan_required / replan
14. review triage required -> review_required / triage_review
15. review repair required within budget -> review_required / repair
16. review dirty over budget or boundary replan_required -> replan_required / replan
17. in-progress normal step or graph task -> continue_required / finish_task
18. ready normal step -> continue_required / run_step
19. ready graph wave -> continue_required / run_wave
20. close gates met and mission-close review required/not passed -> close_required / close_review
21. close gates met and close review not required or passed -> close_required / close_complete
22. no autonomous action but not complete -> explain_and_stop
```

Close and review states are above ordinary execution only when they are required
by the locked plan/state and the loop is active and unpaused. Inactive and
paused loops win first.

## State Against Locked Plan

Status must compare mutable state maps against the current locked plan.

Invalid state if:

- normal plan has step IDs missing from `STATE.steps`
- `STATE.steps` has non-superseded IDs not in locked normal plan
- graph plan has task IDs missing from `STATE.tasks`
- `STATE.tasks` has non-superseded IDs not in locked graph plan
- required review boundaries from plan are missing from `STATE.reviews`
- state records have mismatched IDs

This prevents the close-gates bug where deleting `STATE.tasks.T1` made an
unfinished graph mission look complete.

## Normal Execution

Normal mode is serial checklist execution.

Status:

- if any step is `in_progress`, next action is `finish_task` for that step
- otherwise first pending step in plan order is `run_step`
- all steps complete means close projection

`task start <step-id>`:

- requires step is current ready step
- sets status to `in_progress`
- records `started_revision`

`task finish <step-id>`:

- requires step is `in_progress`
- validates proof file
- sets status to `complete`
- records `completed_revision`, `proof_digest`, `proof_path`

## Graph Execution

Graph mode is first-class product behavior.

Ready frontier:

```text
ready task = pending task whose dependencies are satisfied
dependency satisfied = dependency task complete and required current review boundary for that dependency is passed
```

Do not enforce hidden topological-layer barriers unless modeled as explicit
dependencies or review/integration tasks.

Recommended wave:

```text
input = ready frontier
output = maximal safe subset, or serial fallback if uncertain
```

Wave safety must consider:

- `parallel_safe`
- `unknown_side_effects`
- `exclusive_resources`
- write path overlap
- broad write globs
- package/global/config/schema/database/migration files
- generated files or lockfiles
- tasks already in progress
- currently active wave reservations

If two tasks conflict on `write_paths`, they cannot be in the same safe wave.
Treat globs conservatively. If unsure, put only one task in the wave.

The status field `ready_tasks` may list the full ready frontier. The field
`ready_wave` is the scheduler-approved safe subset. `task start` must obey
`ready_wave`, not raw `ready_tasks`.

### Starting Parallel Waves

Starting one member of a safe wave must not make the remaining safe wave members
unstartable.

Implement one of:

```text
active_wave: {
  "wave_id": "wave_...",
  "task_ids": ["T1", "T2"],
  "started_task_ids": ["T1"],
  "plan_digest": "sha256:...",
  "outcome_digest": "sha256:...",
  "created_revision": 12
}
```

or an equivalent reservation model.

Rules:

- when first task in a wave starts, reserve that wave
- remaining tasks in the same reserved wave remain startable
- tasks excluded from the wave remain unstartable while conflicts are in
  progress
- if a task in the wave completes, remaining unstarted wave tasks can still
  start if no new conflict invalidates the wave
- if plan digest or state revision constraints invalidate the wave, clear it and
  recompute safely

This prevents two opposite bugs:

- parallel wave members becoming impossible to start after the first member
- unsafe excluded tasks starting outside the recommended wave while another task
  is in progress

## Proof Records

Proof records are JSON files under `proofs/`.

```json
{
  "schema_version": "codex1.proof.v1",
  "mission_id": "demo",
  "subject_id": "S1",
  "subject_kind": "step",
  "recorded_revision": 12,
  "outcome_digest": "sha256:...",
  "plan_digest": "sha256:...",
  "commands": [
    {
      "command": "cargo test",
      "exit_code": 0,
      "summary": "passed"
    }
  ],
  "manual_checks": [
    "Inspected diff for unrelated changes."
  ],
  "changed_files": [
    "src/lib.rs"
  ],
  "notes": "..."
}
```

Validation:

- schema version exact
- mission ID matches
- subject ID and kind match the step/task being finished
- `outcome_digest` equals current ratified outcome digest
- `plan_digest` equals current locked plan digest
- `recorded_revision` is an integer
- `recorded_revision <= current STATE.revision`
- `recorded_revision >= started_revision` for the subject
- command records have integer exit codes
- proof path is inside `proofs/` or is copied/imported under `proofs/`

Stale proof digests cannot finish a task.

## Review Records

Review records live under `reviews/`.

```json
{
  "schema_version": "codex1.review.v1",
  "review_id": "R-004",
  "mission_id": "demo",
  "boundary_id": "RB-T1",
  "boundary_revision": 14,
  "outcome_digest": "sha256:...",
  "plan_digest": "sha256:...",
  "raw_output": {
    "findings": [],
    "overall_correctness": "patch is correct",
    "overall_explanation": "...",
    "overall_confidence_score": 0.85
  },
  "adjudication": {
    "findings": [
      {
        "raw_finding_index": 0,
        "decision": "accepted_blocking",
        "reason": "Violates AC-2."
      }
    ]
  }
}
```

Raw findings are observations, not work.

Adjudication decisions:

```text
accepted_blocking
accepted_deferred
rejected
duplicate
stale
```

Triage rules:

- if raw findings are present, every raw finding must have exactly one
  adjudication
- adjudication indexes must be in range
- duplicate adjudication indexes are invalid
- missing adjudication keeps boundary `triage_required`
- unadjudicated raw finding cannot be ignored
- boundary passes only when all raw findings are adjudicated and accepted
  blocking count is zero
- accepted blocking count > 0 sets `repair_required`
- persisted review records must include non-null `boundary_revision`,
  `outcome_digest`, and `plan_digest`

This prevents partial adjudication from passing review blockers.

## Repair And Replan

Repair is per review boundary.

Rules:

- default max repair rounds is 2 unless boundary specifies another value
- repair blockers as one batch
- `review repair-record` increments `repair_round` exactly once per batch
- targeted re-review does not reset repair budget
- dirty after max repair rounds projects `replan_required`
- `replan.required` projects `replan_required` before normal execution
- `plan lock --replan` is the controlled relock path

Replan is not a separate product. It is part of the main Codex1 product.

## Close Readiness And Closeout

Close readiness is derived. It does not trust `STATE.close.state` alone.

Pre-close gates:

- outcome ratified
- current `OUTCOME.md` digest equals ratified digest
- plan locked
- current `PLAN.yaml` digest equals locked digest
- state step/task maps exactly match locked plan work items
- all required steps/tasks complete or superseded by explicit replan semantics
- no accepted blocking review findings
- no boundary in triage/repair/replan state
- no `replan.required`
- no active unsafe graph wave
- terminal not already complete

Mission-close review:

- if `PLAN.yaml.requires_mission_close_review == true`, close readiness projects
  `close_review` until that review passes
- mission-close review cannot be skipped by setting close state manually
- `close record-review` records the review result and anchors it to current
  `outcome_digest`, `plan_digest`, and revision

`close complete`:

- requires close gates met
- requires mission-close review passed if required
- writes or rewrites `CLOSEOUT.md` for the current pre-terminal revision
- does not trust stale existing closeout content
- validates closeout frontmatter before terminalizing
- stores closeout digest
- records terminal completion

Closeout frontmatter:

```yaml
schema_version: codex1.closeout.v1
mission_id: demo
pre_terminal_revision: 17
outcome_digest: sha256:...
plan_digest: sha256:...
completed_work:
  - S1
proof_summary:
  - cargo test passed
review_summary:
  - RB-T1 passed
```

`close_complete` records `terminal.completed_revision = pre_terminal_revision +
1`.

Closeout path is exactly `CLOSEOUT.md`.

## Ralph Stop Hook

Ralph is a thin adapter over status.

Official Codex Stop-hook inputs include `cwd`, `model`, `permission_mode`,
`stop_hook_active`, `turn_id`, and transcript metadata in the local official
Codex repo. Verify against:

```text
/Users/joel/.codex/.codex-official-repo
```

Ralph algorithm:

```text
1. parse Stop-hook JSON from stdin
2. if stop_hook_active == true, emit allow output and exit 0
3. use hook input cwd to resolve repo/mission when explicit CLI flags are absent
4. call status projector
5. fail open on any parse, resolution, state, artifact, or status error
6. fail open unless loop active, unpaused, and mode is execute/autopilot/review_loop
7. fail open unless status.stop.message is non-empty
8. fail open unless next_action.owner == codex
9. fail open unless next_action.required == true
10. fail open unless next_action.autonomous == true
11. fail open unless next_action.kind is in the block allowlist
12. emit decision:block with reason/message
```

Block allowlist:

```text
run_step
run_wave
finish_task
triage_review
repair
replan
close_review
close_complete
```

Unknown/future next actions fail open.

Block message must include:

```text
Continue the active work, or use $interrupt / codex1 loop pause to pause intentionally.
If this is wrong, briefly explain and stop; Ralph will allow the next stop.
```

Ralph has no local state. No `.ralph` files.

## Doctor

Doctor is fast, read-only, and diagnostic.

Required checks:

- installed `codex1` command works from outside source checkout
- JSON error path works from outside source checkout
- Stop-hook input schema contains expected fields in official Codex repo
- Stop-hook output schema supports `decision:block`, `reason`, and `continue`
- TOML hook config shape supports `Stop`, `command`, `timeout`, and
  `statusMessage` in official Codex repo
- `codex_hooks` feature exists and is default-on in official Codex repo
- bundled model catalog contains `gpt-5.5` and `gpt-5.4-mini`
- state/event drift check handles missing, malformed, non-object, and unreadable
  event logs without crashing

Doctor `ok` is false if required checks fail. Use `warning` and `info` only for
genuinely optional diagnostics.

`doctor --e2e` may run slower or invasive checks, such as subagent hook-disable
proof. Default doctor must remain fast and non-invasive.

## Repo Discovery

Read paths:

- explicit `--repo-root` wins
- otherwise walk from cwd to find `.git`
- otherwise walk from cwd to find ancestor containing `PLANS/`
- if cwd is inside `PLANS/<mission-id>/`, resolve that mission
- if no mission in cwd, read `PLANS/ACTIVE.json` if valid

Write paths:

- ordinary mutating commands require explicit `--mission`
- do not silently select a mission from `ACTIVE.json`
- exceptions: `init`, explicit selection/activation commands, and commands whose
  whole purpose is changing active loop state if explicitly documented

## Implementation Architecture

Use small modules with clear ownership:

```text
src/cli/            argument parsing and command dispatch only
src/errors/         stable result and error envelopes
src/schema/         typed structs and validators
src/paths/          repo and mission resolution, symlink safety
src/artifacts/      OUTCOME, PLAN, PROOF, REVIEW, CLOSEOUT parsers and digests
src/state_store/    lock, transaction, atomic write, events
src/status/         status projector and stop projector
src/plan/           plan and graph validation
src/graph/          ready frontier and safe wave scheduler
src/tasks/          task start/finish state transitions
src/reviews/        review, triage, repair, replan transitions
src/close/          close gates and closeout generation
src/ralph/          Stop-hook adapter
src/doctor/         diagnostics
tests/              unit, integration, CLI, regression
```

The exact language can vary, but the boundaries must exist.

Do not put the product in one giant file.

## Subagent Ownership Map

The orchestrating implementation thread should split work into tiny owned
slices. Each subagent reads this file first and owns one slice.

Recommended ownership:

1. schemas/errors: JSON envelopes, enums, typed state structs, validators
2. paths: mission IDs, repo discovery, ACTIVE pointer, symlink/path escapes
3. artifacts: YAML/frontmatter parsing, canonical digests, proof/review/closeout
4. state store: locks, atomic writes, events, revision conflicts
5. plan/graph: plan validation, graph cycles, frontier, safe waves, active wave
6. status/close: status priority, close gates, stop projection
7. tasks/proofs: task start/finish, proof anchoring, state-plan matching
8. reviews/replan: review records, complete triage, repair budget, replan relock
9. ralph/doctor: Stop-hook adapter and Codex official repo diagnostics
10. cli/package: command parsing, JSON parse errors, installed smoke tests
11. regression tests: adversarial fixtures covering every finding in this file

Subagents are not alone in the codebase. They must not revert others' work.
They must coordinate through module interfaces and tests.

## Regression Test Matrix

The implementation is not done until tests cover all of these.

Path and resolution:

- invalid mission IDs rejected
- absolute mission ID rejected
- dot-dot mission ID rejected
- symlinked mission root outside `PLANS/` rejected
- `STATE.json.mission_id` mismatch projects invalid state
- non-object `ACTIVE.json` ignored with warning
- cwd inside `PLANS/<mission-id>/` resolves status
- non-git tree containing `PLANS/` resolves active mission from subdirs

JSON contract:

- `--json` argument errors return `codex1.error.v1`
- revision conflict returns `REVISION_CONFLICT` with `current_revision`
- no Python/Rust/stack trace appears in JSON mode

Outcome:

- fresh empty scaffold fails `outcome check`
- empty title/original/destination/acceptance/proof/constraints/non_goals fail
- explicit none item for constraints/non_goals passes
- ratify stores outcome digest
- editing outcome after ratify projects invalid state

Plan:

- normal plan validates and locks
- graph plan validates and locks
- missing dependency rejected
- duplicate dependency rejected
- cycle rejected
- duplicate task/step IDs rejected
- invalid write path rejected
- lock initializes all planned state work items
- plain lock rejects already locked plan
- edited plan after lock projects invalid state
- replan lock requires `replan.required`

Transactions:

- artifact writes do not happen when lock acquisition fails
- event append failure after state commit returns ok with warning
- missing event log with revised state reports drift
- malformed event row reports drift
- non-object state projects invalid state
- invalid booleans project invalid state and Ralph allows stop
- invalid revision projects invalid state and Ralph allows stop

Loop:

- loop activate requires ratified outcome and locked plan
- activation pointer write failure is coherent and not a false failed mutation
- inactive loop wins before replan/close
- paused loop wins before replan/close
- loop mode none never blocks Ralph

Normal execution:

- active locked pending step projects `run_step`
- starting current step succeeds
- in-progress step projects `finish_task`
- stale proof digest rejected
- proof recorded before start rejected
- finishing S1 projects S2 in a two-step plan
- all steps complete projects close

Graph:

- ready frontier respects dependencies
- close gates reject missing state task from locked plan
- ready wave excludes exclusive resource conflicts
- ready wave excludes write path conflicts
- broad/unknown writes force serial fallback
- starting T1 in `[T1,T2]` safe wave keeps T2 startable
- task excluded from wave cannot start while conflicting task in progress
- in-progress graph task projects finish without losing active wave reservation

Review/replan:

- keyed review blockers prevent close
- raw findings without adjudication keep `triage_required`
- duplicate adjudication indexes invalid
- partial adjudication cannot pass boundary
- accepted blocking sets `repair_required`
- repair round increments exactly once
- dirty after max repair rounds projects `replan_required`
- persisted review record has non-null freshness anchors
- replan relock clears replan and supersedes old route safely

Close:

- close readiness derived from locked plan and state
- mission-close review required projects `close_review`
- close complete rewrites stale closeout
- stale closeout pre-terminal revision rejected
- closeout path escape rejected
- terminal completion revision matches closeout semantics

Ralph:

- uses Stop-hook `cwd` to resolve mission
- stop_hook_active true allows
- active execute/autopilot/review_loop blocks safe required work
- unknown next action allows
- invalid state allows
- inactive/paused/mode none allows
- block message includes interrupt/pause escape hatch

Doctor:

- installed command checked from temp cwd with source-local env removed
- required Codex integration check failure makes doctor ok false
- event drift diagnostics do not crash on unreadable or malformed files

Packaging:

- installed command works from outside checkout
- `.DS_Store` ignored and untracked

## Verification Commands

Use the repo's language-specific commands, plus these conceptual checks:

```bash
codex1 --json definitely-invalid-command
codex1 --help
codex1 status --json
codex1 doctor --json
```

Run an installed-command smoke test from a temporary directory, not from the
source checkout.

## Definition Of Done

Done means:

- all commands in the chosen implementation surface are real or clearly return
  stable `NOT_IMPLEMENTED` without pretending success
- the whole product architecture is represented in schemas and status even if
  some mutation paths are added later
- normal mode works end to end
- graph validation and status are real if graph plans can lock
- review/replan status and record semantics are real if review commands exist
- Ralph is conservative and fail-open
- every prior review finding has a regression test
- the installed CLI works from outside the checkout
- the code is modular enough that another engineer can read one subsystem
  without loading the entire product into their head
