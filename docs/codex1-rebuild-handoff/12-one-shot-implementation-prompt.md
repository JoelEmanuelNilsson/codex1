# 12 One-Shot Implementation Prompt

This file is a fresh-thread handoff prompt for rebuilding Codex1 in a new repo.

It exists because the first coding attempt exposed the real failure mode:

```text
The problem was not one bug.
The problem was implementing command breadth before invariant depth.
```

Use this file when starting a new Codex thread in a new repo. The prompt below is
intended to be pasted as the user message. It should produce a complete first
product CLI implementation, excluding the user-facing skills themselves.

The prompt is intentionally long. It is meant to prevent the next agent from
filling gaps with vibes.

---

# Copy-Paste Prompt Starts Here

You are building **Codex1**, a skills-first durable workflow substrate for native
Codex.

You are in a fresh repository. Implement the product described below. Do not
copy old source code from any previous Codex1 implementation attempt. You may
read the documentation folder if it is present, but this prompt is the
implementation authority for this run.

Before coding, read and follow the local `cli-creator` skill:

```text
/Users/joel/.codex/skills/cli-creator/SKILL.md
```

Use the skill for CLI design discipline: composable commands, stable JSON,
machine-readable errors, an installed command that works from outside the source
checkout, `doctor`, and a final smoke test from `/tmp`.

Do **not** write the Codex skills in this task. Do everything else needed for the
CLI/product substrate. The skills will be written in a later thread.

Use the auto-updated official Codex source as the source of truth for Codex
internals:

```text
/Users/joel/.codex/.codex-official-repo
```

When verifying Codex Stop-hook, model catalog, hook config, or review protocol
claims, inspect that local official repo. Do not rely on memory.

## The Product

Codex1 is not a replacement for Codex.

Codex stays the actor. Codex1 gives Codex durable rails when durable rails are
worth it:

```text
skills-first UX
deterministic codex1 CLI
visible durable mission files under PLANS/<mission-id>/
status --json as the integration surface
Ralph as a thin Stop-hook guard over status
normal planning for ordinary durable work
graph planning for large/risky/multi-agent work
planned review boundaries
repair and replan loops
mission-close review
```

The user should experience:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

But again: do not write those skills in this task. Build the CLI and durable
substrate those skills will call.

## The Hard Lesson From The Failed Branch

The previous coding attempt spiraled because it implemented many command names
and status branches before making the invariant core impossible to violate.

Review kept finding new issues:

- state wrote before event append and crashed after partial commit
- artifact writes happened before acquiring the mission lock
- stale closeout evidence could terminalize the mission
- graph plans could lock even though graph status could not execute them
- graph dependencies were ignored in status
- empty outcomes ratified
- mutating commands used hidden active mission selection
- loop activation succeeded without a locked plan
- noncanonical verdicts leaked
- closeout paths could escape the mission directory
- status mixed mutable state with derived readiness
- Ralph could block from malformed or unsafe projections

Those are not random bugs. They are one pattern:

```text
Feature surface was implemented before the invariant bundle that makes it safe.
```

This implementation must invert that order.

## Non-Negotiable Implementation Law

Do not implement commands first.

Implement this spine first:

```text
typed artifact parsers
canonical digests
mission id/path validation
state schema validation
transaction engine
plan validator
graph validator
review state validator
status projector
stop projector
close readiness engine
doctor checks
then command adapters
```

Commands should be thin wrappers. If a command needs to know readiness,
dependency satisfaction, close gates, or stop behavior, it must call shared
engines. It must not rederive truth.

## One Product, No Fake Deferral

Graph, review, repair, replan, and mission-close review are part of the first
product. They are not a separate product. Do not delete them, do not rename them
as an optional extension, and do not write docs that imply they are out of
scope.

But do not create halfway doors.

The rule is:

```text
A capability may be exposed in help before use, but it must not accept durable
state until its full invariant bundle is implemented.
```

For this task, implement the full CLI substrate for:

- normal mode
- graph mode
- planned review boundaries
- repair recording
- replan recording and relock
- mission-close review
- Ralph Stop-hook guard
- doctor checks

Do not implement the Codex skill files. Do not implement subagent spawning.
The CLI prepares packets and records results; Codex and the later skills decide
when to spawn workers/reviewers.

## Runtime Choice

Prefer **Rust** for this rebuild unless the repo/toolchain makes Rust clearly
wrong.

Reason:

- durable CLI
- strong enums and schema validation
- good JSON/YAML/TOML support
- easy `~/.local/bin/codex1` install
- less temptation to accept arbitrary dynamic JSON

Recommended Rust crates:

```text
clap
serde
serde_json
serde_yaml
toml
anyhow or thiserror
camino or camino-like UTF-8 paths if useful
tempfile
fs2 or another small locking crate if needed
sha2
chrono/time
uuid or deterministic revision-derived IDs
```

If Rust is unavailable, choose the best installed runtime after following
`cli-creator`, but keep the architecture identical.

The binary name is:

```text
codex1
```

Before scaffolding, check:

```bash
command -v codex1 || true
command -v cargo rustc python3 uv node npm pnpm || true
```

If `codex1` already exists on PATH, your local build must still install or expose
the new one clearly for tests. Do not accidentally test a stale installed
binary.

## Definition Of Done

The product is done for this task when:

1. `codex1 --help` shows the integrated command surface.
2. `codex1 --json doctor` runs from outside the source checkout.
3. A full durable normal mission can be created, ratified, planned, locked,
   activated, executed, and closed.
4. A valid graph plan can be locked and status derives dependency-safe ready
   frontiers/waves.
5. Invalid graph plans cannot lock.
6. Planned review boundaries can be started, packeted, recorded, triaged,
   repaired, re-reviewed, marked passed, or escalated to replan.
7. Replan can be recorded and relocked without reusing old task IDs.
8. Mission-close review gates terminal close when required.
9. Ralph uses Stop-hook `cwd`, blocks only safe autonomous next actions, and
   fails open on invalid/corrupt/unknown state.
10. All writes are lock-protected and transactionally ordered.
11. Stable JSON errors are emitted for every failure path.
12. Tests cover the failures that killed the previous branch.
13. The installed `codex1` command works from `/tmp` or another directory outside
    the source checkout.

## Product Mental Model

The product loop is:

```text
User invokes a skill.
Skill decides chat-only, durable normal, or graph.
CLI records deterministic durable truth when needed.
status --json projects the one next action.
Codex executes until close, interrupt, invalid state, or explain-and-stop.
Ralph only blocks active unpaused autonomous continuation.
```

The user should not feel like they are operating a workflow database.

The CLI should feel boring, exact, and trustworthy.

## Architecture

Build a small core library and thin command wrappers.

Recommended module boundaries:

```text
src/
  main / cli
    argument parsing
    command routing
    JSON formatting
    stable error formatting

  model / schema
    typed structs/enums for every durable artifact
    typed status envelope
    typed command result envelope
    typed error envelope

  paths / resolver
    repo root discovery
    mission id validation
    mission root validation
    active pointer loading
    Ralph cwd resolution

  artifacts
    OUTCOME.md frontmatter parser/writer
    PLAN.yaml parser/writer
    STATE.json parser/writer
    EVENTS.jsonl append/read helpers
    reviews/*.json parser/writer
    proof parser/writer
    CLOSEOUT.md parser/writer
    canonical digest

  transaction
    mission lock
    atomic state write
    artifact write staging
    event append
    warnings/audit drift on event append failure

  outcome
    outcome validation
    ratification transition

  plan
    plan validation
    normal plan validation
    graph plan validation
    replan relock validation

  graph
    cycle detection
    dependency satisfaction
    ready frontier
    recommended wave
    serial fallback when safety unclear

  status
    normalized mission snapshot
    status priority table
    next_action projection
    close readiness
    stop projection

  task
    task/step lifecycle
    proof validation
    task packets

  review
    review boundary lifecycle
    review packet generation
    raw review normalization
    triage/adjudication recording
    repair budget
    repair-record transition

  replan
    replan required state
    replan record
    plan lock --replan

  close
    close check
    mission-close review record
    closeout generation
    terminalization

  ralph
    Stop-hook stdin parse
    cwd-based mission resolution
    status consumption
    official output

  doctor
    installed command proof
    official Codex repo checks
    hook config checks
    model catalog checks
    state/event drift diagnostics
```

No command should hand-roll path selection, state writes, status projection, or
close readiness.

## Durable File Layout

Every durable mission lives here:

```text
PLANS/
  ACTIVE.json
  <mission-id>/
    OUTCOME.md
    PLAN.yaml
    STATE.json
    EVENTS.jsonl
    specs/
      <task-id>/
        SPEC.md
        PROOF.md
        REPAIR-<n>.md
    reviews/
      R-0001.json
      R-0001-packet.json
    CLOSEOUT.md
```

`PLANS/ACTIVE.json` is repo-level selector metadata only. It is not mission
truth.

Normal-mode missions may not use `specs/` and `reviews/` much, but the
directories may exist.

## Mission ID And Path Rules

Mission IDs are durable IDs under `PLANS/<mission-id>`.

Valid mission IDs:

```text
lowercase letters
numbers
dash
underscore
dot only if not first/last and not repeated as path traversal
length 1..80
```

Recommended regex:

```text
^[a-z0-9][a-z0-9._-]{0,79}$
```

Additional validation:

- reject absolute paths
- reject `/`
- reject `\`
- reject `..`
- reject empty
- canonicalize `repo_root/PLANS/<mission-id>`
- verify the result remains under `repo_root/PLANS`
- verify `STATE.json.mission_id` matches the selected mission id

Never trust a mission id from CLI args, `ACTIVE.json`, or `STATE.json` until it
passes these checks.

## Repo Root And Mission Resolution

For ordinary commands:

- Read commands may use explicit args, cwd mission discovery, or active pointer.
- Mutating commands must require explicit `--mission <id>` and either explicit
  `--repo-root <path>` or discoverable repo root.
- The only exceptions are commands whose purpose is creating/selecting/activating
  a mission:
  - `codex1 init`
  - `codex1 loop activate`

For Ralph:

1. Parse Stop-hook stdin.
2. If `stop_hook_active == true`, allow stop immediately.
3. Use Stop-hook `cwd` to resolve the mission when explicit flags are absent.
4. If `cwd` is inside `PLANS/<mission-id>/`, select that mission directly.
5. Otherwise find nearest ancestor containing `PLANS/`.
6. Read `PLANS/ACTIVE.json`.
7. Validate pointer schema and mission id.
8. Validate selected mission state identity.
9. Project status.
10. Fail open on anything invalid, missing, ambiguous, or unsupported.

Do not scan all missions and guess.

## JSON Output Policy

Every command supports `--json`.

Success envelope:

```json
{
  "ok": true,
  "schema_version": "codex1.result.v1",
  "command": "plan lock",
  "mission_id": "demo",
  "state_revision": 4,
  "data": {},
  "warnings": []
}
```

Error envelope:

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "REVISION_CONFLICT",
  "message": "Expected state revision 17 but current revision is 18.",
  "retryable": true,
  "current_revision": 18,
  "warnings": []
}
```

Rules:

- Never print a Python/Rust/JS traceback as command output.
- Never include secrets.
- Use finite stable error codes.
- Include `current_revision` for revision conflicts.
- Include `retryable` for lock/revision conflicts.
- A command that commits state but cannot append the audit event should return a
  stable result with a warning and `audit_drift: true`; it must not crash after
  committing state.

Recommended error codes:

```text
ARGUMENT_ERROR
MISSION_NOT_FOUND
MISSION_ID_INVALID
MISSION_MISMATCH
MISSION_LOCKED
REVISION_CONFLICT
SCHEMA_UNSUPPORTED
ARTIFACT_MISSING
ARTIFACT_INVALID
DIGEST_MISMATCH
OUTCOME_INCOMPLETE
OUTCOME_NOT_RATIFIED
PLAN_INVALID
PLAN_NOT_LOCKED
PLAN_ALREADY_LOCKED
PLAN_LOCK_REQUIRED
GRAPH_INVALID
TASK_NOT_FOUND
TASK_NOT_READY
TASK_ALREADY_STARTED
TASK_NOT_IN_PROGRESS
PROOF_REQUIRED
REVIEW_NOT_FOUND
REVIEW_NOT_READY
REVIEW_TRIAGE_REQUIRED
REPAIR_NOT_READY
REPLAN_NOT_REQUIRED
CLOSE_NOT_READY
CLOSE_REVIEW_REQUIRED
CLOSEOUT_INVALID
DOCTOR_FAILED
NOT_IMPLEMENTED
INTERNAL_ERROR
```

`NOT_IMPLEMENTED` should be used only for truly unimplemented future helper
surface. For this task, the integrated product commands below should not return
`NOT_IMPLEMENTED`.

## Canonical Digest Rules

Digests must be computed from canonical machine-parsed content, not raw bytes.

Use:

```text
sha256:<hex>
```

Canonical serialization:

- parse YAML/JSON/frontmatter into typed data
- reject unsupported schema versions
- normalize to a JSON-compatible value
- sort object keys recursively
- preserve array order
- use stable JSON without insignificant whitespace
- hash UTF-8 bytes of that stable JSON

Digest domains:

```text
OUTCOME.md digest = canonical frontmatter object only
PLAN.yaml digest = canonical parsed plan object
CLOSEOUT.md digest = canonical closeout frontmatter plus body, or a documented typed closeout object including summary/proof fields
review record digest = canonical review JSON object
proof digest = canonical proof frontmatter/object
```

Formatting-only edits to markdown body after `OUTCOME.md` frontmatter should not
change outcome digest unless that body is declared part of the structured
machine truth. For v1, parse only frontmatter as machine truth.

Test:

- YAML key reorder gives same digest.
- Whitespace-only YAML changes give same digest.
- Semantic field change gives different digest.
- Editing OUTCOME after ratify projects invalid_state.
- Editing PLAN after lock projects invalid_state.

## Transaction Model

Every mutating command uses one transaction helper.

Pseudocode:

```text
resolve repo root and mission id
validate mission id/path
load lightweight current state for preflight
validate explicit --expect-revision if required or provided
acquire mission-local lock via atomic create
under lock:
    re-read STATE.json
    validate schema and integer revision
    re-check expected revision
    re-parse relevant artifacts
    re-check transition preconditions
    stage artifact writes if any
    stage next state
    write artifact temp files in mission directory
    atomically rename artifact temp files
    write STATE.json temp file in mission directory
    atomically rename STATE.json
    append one EVENTS.jsonl line
    if event append fails:
        keep committed state
        return ok true with warning audit_drift true
release lock
return stable JSON
```

Mission-local lock:

```text
PLANS/<mission-id>/.codex1.lock
```

Rules:

- acquire with atomic create
- never auto-delete stale locks in ordinary commands
- if lock exists, return retryable `MISSION_LOCKED`
- an explicit future repair command may inspect/remove stale locks, but Ralph
  must never repair locks
- lock file contains command, pid, hostname, started_at

Artifact writes:

- No command may modify `PLAN.yaml`, `OUTCOME.md`, `CLOSEOUT.md`, review files,
  proof files, or specs before acquiring the mission lock if the command also
  mutates state.
- If validation fails, do not mutate artifacts.
- If lock acquisition fails, do not mutate artifacts.
- If state revision check fails, do not mutate artifacts.
- Use temp files inside the mission directory and atomic rename.

Event append failure:

- `STATE.json` remains authoritative.
- command must not traceback
- command result includes warning:

```json
{
  "code": "AUDIT_EVENT_APPEND_FAILED",
  "message": "STATE.json committed at revision 5 but EVENTS.jsonl append failed.",
  "audit_drift": true
}
```

- `doctor` must report audit drift later.

## Active Pointer

`PLANS/ACTIVE.json`:

```json
{
  "schema_version": "codex1.active.v1",
  "mission_id": "demo",
  "selected_at": "2026-04-24T10:00:00Z",
  "selected_by": "codex1 loop activate",
  "purpose": "ralph_status_default"
}
```

Rules:

- Read/status/Ralph may use it.
- Ordinary mutating commands must not silently use it.
- `loop activate --mission <id>` may write it.
- `loop deactivate --mission <id>` may clear it only if it points to that
  mission.
- Invalid/non-object/stale/mismatched active pointer produces a warning or
  fail-open status, not a crash.

## OUTCOME.md Schema

Writers emit YAML frontmatter plus optional markdown body:

```markdown
---
schema_version: codex1.outcome.v1
mission_id: demo
title: "Implement durable Codex1 workflow substrate"

original_user_goal: |
  User's original words or a faithful quote/summary.

destination: |
  Concrete destination, written so a future Codex thread can continue without
  hidden chat context.

acceptance_criteria:
  - "A durable mission can be initialized, planned, executed, reviewed, replanned, and closed through codex1."

constraints:
  - "Codex stays the actor; codex1 is deterministic substrate."

non_goals:
  - "Do not build a wrapper runtime around Codex."

proof_expectations:
  - "Unit tests and end-to-end CLI smoke tests pass."

pr_intent:
  open_pr: false
  target_branch: null
  notes: "Open a PR only if clarified by the user."

must_be_true: []
definitions: {}
quality_bar: []
review_expectations: []
known_risks: []
user_only_actions: []
resolved_questions: []
---

Optional human notes here.
```

Required fields:

```text
schema_version == codex1.outcome.v1
mission_id
title
original_user_goal
destination
acceptance_criteria
constraints
non_goals
proof_expectations
pr_intent.open_pr
```

Optional fields may be empty, but placeholders are invalid.

Reject:

- empty title
- empty destination
- empty acceptance criteria
- acceptance criteria with vague placeholder text
- empty proof expectations
- mission_id mismatch
- unsupported schema
- frontmatter not an object
- boilerplate template markers such as `TODO`, `...`, `TBD`, `fill me`, `replace`

Ratification truth lives in `STATE.json`, not `OUTCOME.md`. Do not use an
authoritative `status: ratified` field in `OUTCOME.md`.

## PLAN.yaml Schema

Common shape:

```yaml
schema_version: codex1.plan.v1
mission_id: demo
mode: normal            # normal | graph
level: medium           # small | medium | large | risky
title: "..."
outcome_digest: "sha256:..."

acceptance_criteria:
  - id: AC1
    text: "..."

validation:
  commands:
    - "cargo test"
  manual_checks: []

requires_mission_close_review: false

normal:
  steps: []

graph:
  tasks: []
  review_boundaries: []
```

Exactly one of `normal.steps` or `graph.tasks` is required according to `mode`.

### Normal Plan

```yaml
schema_version: codex1.plan.v1
mission_id: demo
mode: normal
level: medium
title: "Normal durable mission"
outcome_digest: "sha256:..."
requires_mission_close_review: false

acceptance_criteria:
  - id: AC1
    text: "The CLI can complete the workflow."

validation:
  commands:
    - "cargo test"
  manual_checks:
    - "Inspect status JSON for canonical next_action."

normal:
  steps:
    - id: S1
      title: "Implement artifact parser"
      goal: "Parse OUTCOME.md and PLAN.yaml into typed objects."
      acceptance:
        - "Reject empty outcome."
      proof:
        required: true
        path: "specs/S1/PROOF.md"
        commands:
          - "cargo test artifact_parser"
      write_paths:
        - "src/artifacts/**"
        - "tests/artifacts/**"
```

Normal step validation:

- IDs unique
- ID regex `^S[0-9]+$`
- title non-empty
- goal non-empty
- acceptance non-empty
- proof.required boolean
- proof.path is relative and inside mission root if present
- write_paths are relative repo paths or globs
- no `depends_on`
- no graph-only fields

Normal state uses `STATE.json.steps`.

### Graph Plan

```yaml
schema_version: codex1.plan.v1
mission_id: demo
mode: graph
level: risky
title: "Graph durable mission"
outcome_digest: "sha256:..."
requires_mission_close_review: true

acceptance_criteria:
  - id: AC1
    text: "Graph dependencies are respected."

validation:
  commands:
    - "cargo test"
  manual_checks: []

graph:
  tasks:
    - id: T1
      title: "Implement state schema"
      goal: "Typed state model and validation."
      depends_on: []
      acceptance:
        - "Invalid revision is rejected."
      proof:
        required: true
        path: "specs/T1/PROOF.md"
        commands:
          - "cargo test state_schema"
      spec_path: "specs/T1/SPEC.md"
      write_paths:
        - "src/state/**"
      exclusive_resources: []
      unknown_side_effects: false
      parallel_safe: true

    - id: T2
      title: "Implement status projector"
      goal: "Project the one canonical next action."
      depends_on: [T1]
      acceptance:
        - "Status respects dependency satisfaction."
      proof:
        required: true
        path: "specs/T2/PROOF.md"
        commands:
          - "cargo test status_projector"
      spec_path: "specs/T2/SPEC.md"
      write_paths:
        - "src/status/**"
      exclusive_resources: []
      unknown_side_effects: false
      parallel_safe: false

  review_boundaries:
    - id: RB-T1-1
      target:
        tasks: [T1]
        files:
          - "src/state/**"
      required: true
      max_repair_rounds: 2
      reviewer_prompt: "Review the T1 boundary for correctness against PLAN.yaml and OUTCOME.md."

    - id: RB-close-1
      target:
        mission: true
      required: true
      max_repair_rounds: 2
      mission_close: true
      reviewer_prompt: "Review the whole mission before terminal close."
```

Graph validation must reject:

- duplicate task IDs
- duplicate dependency entries
- unknown dependencies
- self-dependencies
- cycles
- invalid IDs
- missing required fields
- non-list `depends_on`
- review boundary target with unknown task
- mission-close review boundary missing when `requires_mission_close_review == true`
- absolute paths or escaping paths
- `spec_path` escaping mission root
- `proof.path` escaping mission root

Graph waves:

- Waves are derived, never stored as editable truth.
- `ready_frontier` is all pending tasks whose dependencies are satisfied.
- Dependency satisfaction means:
  - dependency task status is `complete`
  - and all required current review boundaries for that dependency are `passed`
- `recommended_wave` is a safe subset of `ready_frontier`.
- If parallel safety is unclear, recommended wave may be one task.
- Do not impose a hidden topological-layer barrier unless the dependencies say
  so. If an integration barrier is needed, model it as an explicit task.

## STATE.json Schema

Initial state:

```json
{
  "schema_version": "codex1.state.v1",
  "mission_id": "demo",
  "revision": 0,
  "phase": "clarify",
  "planning_mode": "unset",
  "planning_level": {
    "requested": null,
    "effective": null
  },
  "outcome": {
    "ratified": false,
    "ratified_revision": null,
    "outcome_digest": null
  },
  "plan": {
    "locked": false,
    "locked_revision": null,
    "plan_digest": null,
    "outcome_digest_at_lock": null,
    "supersedes": []
  },
  "loop": {
    "active": false,
    "paused": false,
    "mode": "none"
  },
  "steps": {},
  "tasks": {},
  "reviews": {},
  "replan": {
    "required": false,
    "reason": null,
    "boundary_id": null,
    "supersedes": []
  },
  "close": {
    "state": "not_ready",
    "requires_mission_close_review": false,
    "boundary_revision": null,
    "latest_review_id": null,
    "passed_revision": null,
    "closeout_path": "CLOSEOUT.md",
    "closeout_digest": null
  },
  "terminal": {
    "complete": false,
    "completed_revision": null
  }
}
```

Strict state validation:

- top-level JSON object only
- schema_version exact
- mission_id matches selected directory
- revision integer >= 0
- booleans must be booleans, not strings
- phase enum
- planning_mode enum
- loop.mode enum
- every status enum known
- closeout_path must be exactly `CLOSEOUT.md` in v1
- unknown required states project invalid_state in status and fail writes

Task/step status enum:

```text
pending
in_progress
complete
superseded
cancelled
```

Do not use task statuses such as `review_clean` or `review_pending`. Review
cleanliness lives in review boundary state.

Review boundary state enum:

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

Close state enum:

```text
not_ready
ready_for_mission_close_review
mission_close_review_open
mission_close_review_passed
close_complete_ready
terminal_complete
```

Terminal truth lives in `terminal.complete`. `close.state` may mirror
`terminal_complete` for readability but status must treat `terminal.complete` as
the canonical terminal boolean.

## EVENTS.jsonl Schema

One line per committed state revision:

```json
{
  "schema_version": "codex1.event.v1",
  "mission_id": "demo",
  "event_id": "demo:5",
  "revision_before": 4,
  "revision_after": 5,
  "type": "task_finished",
  "actor": "cli",
  "subject": {
    "kind": "task",
    "id": "T1"
  },
  "changes": {
    "tasks.T1.status": ["in_progress", "complete"]
  },
  "created_at": "2026-04-24T10:00:00Z"
}
```

Rules:

- Events are audit, not replay authority in v1.
- Do not reconstruct mission truth from events.
- `doctor` reports if latest event revision is missing, malformed, or behind
  state revision.
- Non-object event rows are malformed.
- Event rows without integer `revision_after` are malformed.

## Proof Schema

Proof files may be markdown with frontmatter or JSON/YAML. Machine truth:

```yaml
schema_version: codex1.proof.v1
mission_id: demo
subject:
  kind: task
  id: T1
recorded_revision: 8
plan_digest: sha256:...
outcome_digest: sha256:...
commands:
  - command: "cargo test state_schema"
    exit_code: 0
    summary: "passed"
manual_checks:
  - "Inspected diff for unrelated changes."
changed_files:
  - "src/state.rs"
notes: |
  Short human proof notes.
```

Task finish requires proof when the plan says proof is required.

Proof path must be relative and inside mission root.

## Review Record Schema

Raw reviewer output should follow official Codex review shape. Verify the
current shape in the official Codex repo before relying on it.

Codex1 persisted review record:

```json
{
  "schema_version": "codex1.review.v1",
  "review_id": "R-0004",
  "mission_id": "demo",
  "boundary_id": "RB-T2-1",
  "boundary_revision": 14,
  "plan_digest": "sha256:...",
  "outcome_digest": "sha256:...",
  "target": {
    "tasks": ["T2"],
    "files": ["src/status/**"],
    "mission": false
  },
  "raw_output": {
    "findings": [],
    "overall_correctness": "patch is correct",
    "overall_explanation": "No blocking issues found.",
    "overall_confidence_score": 0.82
  },
  "adjudication": {
    "decided_by": "main",
    "decided_at_revision": 15,
    "findings": [
      {
        "raw_finding_index": 0,
        "decision": "accepted_blocking",
        "reason": "Violates AC1.",
        "maps_to": ["AC1"]
      }
    ]
  },
  "accepted_blocking_count": 1,
  "accepted_deferred_count": 0,
  "rejected_count": 0,
  "created_at": "2026-04-24T10:00:00Z"
}
```

Finding decisions:

```text
accepted_blocking
accepted_deferred
rejected
duplicate
stale
```

Raw findings are observations. They do not affect status until main/root records
adjudication through `codex1 review record`.

## Closeout Schema

`CLOSEOUT.md` is generated or overwritten by `codex1 close complete` under the
mission lock.

In v1, the closeout path is exactly:

```text
CLOSEOUT.md
```

Do not trust an arbitrary `STATE.json.close.closeout_path`.

Closeout frontmatter:

```yaml
schema_version: codex1.closeout.v1
mission_id: demo
pre_terminal_revision: 18
terminal_revision: 19
outcome_digest: sha256:...
plan_digest: sha256:...
status_digest: sha256:...
completed_at: "2026-04-24T10:00:00Z"
```

Body sections:

```markdown
# Closeout

## Summary

## Completed Work

## Proof

## Review

## Replan History

## Known Deferred Items
```

Rules:

- `close complete` must re-check close readiness under the mission lock.
- It must always generate/overwrite `CLOSEOUT.md` for the current
  pre-terminal revision.
- It must never terminalize an existing stale closeout.
- If an existing closeout is present with the wrong pre_terminal_revision, do
  not reuse it; overwrite or fail with `CLOSEOUT_INVALID`, but never terminalize
  stale evidence.
- `terminal.completed_revision` equals the state revision created by
  `close complete`.

## Status JSON Schema

`codex1 status --json` is the most important command.

Envelope:

```json
{
  "ok": true,
  "schema_version": "codex1.status.v1",
  "repo_root": "/abs/path",
  "mission_id": "demo",
  "mission_root": "/abs/path/PLANS/demo",
  "state_revision": 7,
  "verdict": "continue_required",
  "phase": "execute",
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "outcome": {
    "ratified": true,
    "digest": "sha256:...",
    "fresh": true
  },
  "plan": {
    "locked": true,
    "mode": "graph",
    "level": "risky",
    "digest": "sha256:...",
    "fresh": true
  },
  "ready_steps": [],
  "ready_tasks": ["T2"],
  "ready_wave": {
    "task_ids": ["T2"],
    "parallel_safe": false,
    "reason": "serial fallback: T2 is not parallel_safe"
  },
  "review": {
    "pending_boundaries": [],
    "triage_required": [],
    "repair_required": [],
    "accepted_blocking_count": 0
  },
  "replan": {
    "required": false,
    "boundary_id": null,
    "reason": null
  },
  "close": {
    "ready": false,
    "required": false,
    "requires_mission_close_review": true,
    "mission_close_review_passed": false
  },
  "next_action": {
    "kind": "run_wave",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "task_ids": ["T2"],
    "summary": "Run graph task T2."
  },
  "stop": {
    "allow": false,
    "reason_code": "block_active_graph_wave",
    "message": "Run graph task T2. Use $interrupt or codex1 loop pause to pause intentionally."
  },
  "warnings": []
}
```

Canonical verdict enum:

```text
no_mission
inactive
paused
invalid_state
explain_and_stop
continue_required
replan_required
close_required
complete
```

Do not emit noncanonical verdicts such as `review_required`.

`next_action.kind` enum:

```text
none
explain_and_stop
run_step
run_wave
finish_task
run_review
triage_review
repair
replan
close_review
close_complete
```

Safety fields:

```text
owner: codex | user | none
required: boolean
autonomous: boolean
```

Ralph must only block when these safety fields are safe.

## Status Priority Table

The status projector must be deterministic. Implement the priority order as code
and test it.

Priority:

```text
1. no selected mission -> no_mission / none / stop.allow true
2. unreadable or unsupported state/artifact -> invalid_state / explain_and_stop / stop.allow true
3. STATE.json mission_id mismatch -> invalid_state / explain_and_stop / stop.allow true
4. stale ratified OUTCOME.md digest -> invalid_state / explain_and_stop / stop.allow true
5. stale locked PLAN.yaml digest -> invalid_state / explain_and_stop / stop.allow true
6. terminal.complete true -> complete / none / stop.allow true
7. loop.paused true -> paused / none / stop.allow true
8. loop.active false -> inactive / none / stop.allow true
9. loop.mode none/manual -> inactive / none / stop.allow true
10. outcome not ratified -> explain_and_stop / explain_and_stop / stop.allow true
11. plan not locked -> explain_and_stop / explain_and_stop / stop.allow true
12. replan.required true -> replan_required / replan
13. any current review boundary triage_required -> continue_required / triage_review
14. any current review boundary repair_required and repair_round < max -> continue_required / repair
15. any current review boundary repair_required and repair_round >= max -> replan_required / replan
16. any current review boundary replan_required -> replan_required / replan
17. any in_progress normal step or graph task -> continue_required / finish_task
18. any complete task/step with required not_started review boundary -> continue_required / run_review
19. normal mode: next pending step -> continue_required / run_step
20. graph mode: dependency-satisfied ready frontier -> continue_required / run_wave
21. pre-close gates not met but no work projected -> explain_and_stop / explain_and_stop
22. mission-close review required and not passed -> close_required / close_review
23. close complete ready -> close_required / close_complete
```

Pre-close gates:

- outcome ratified and fresh
- plan locked and fresh
- all normal steps complete, or all graph tasks complete/superseded/cancelled
  according to plan
- no in-progress work
- no current accepted blocking findings
- no triage required
- no repair required
- no replan required
- all required planned review boundaries passed

Status must derive close readiness from these gates. It must not trust
`close.state == close_complete_ready` alone.

## Stop Projection

Status owns stop projection. Ralph adapts it.

Stop blocks only when:

```text
status.ok == true
loop.active == true
loop.paused == false
loop.mode in execute|autopilot|review_loop
next_action.owner == codex
next_action.required == true
next_action.autonomous == true
next_action.kind in block_allowlist
stop.message non-empty
```

Block allowlist:

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
```

Unknown next actions allow stop.

`explain_and_stop` always allows stop.

Block message must include an escape hatch:

```text
Use $interrupt or codex1 loop pause to pause intentionally.
```

## Ralph Stop-Hook Adapter

Command:

```bash
codex1 ralph stop-hook
```

Input: official Codex Stop hook JSON from stdin.

Verify current schema in official Codex repo. Expected fields include:

```json
{
  "cwd": "...",
  "model": "...",
  "permission_mode": "...",
  "stop_hook_active": false,
  "turn_id": "...",
  "transcript_path": null,
  "last_assistant_message": null
}
```

Output:

- `{}` or empty stdout allows stop.
- `{"decision":"block","reason":"..."}` blocks.

Rules:

- malformed stdin -> allow
- non-object stdin -> allow
- `stop_hook_active == true` -> allow
- cannot resolve mission -> allow
- status failure -> allow
- invalid state -> allow
- unknown action -> allow
- unsafe action -> allow
- safe block projection -> block

Ralph must not inspect artifacts directly.

## Command Surface

Implement these commands for this task:

```bash
codex1 init
codex1 status
codex1 doctor

codex1 outcome check
codex1 outcome ratify

codex1 plan choose-mode
codex1 plan choose-level
codex1 plan scaffold
codex1 plan check
codex1 plan lock
codex1 plan graph
codex1 plan waves

codex1 task next
codex1 task start
codex1 task finish
codex1 task status
codex1 task packet

codex1 review start
codex1 review packet
codex1 review record
codex1 review repair-record
codex1 review status

codex1 replan record
codex1 replan check

codex1 loop activate
codex1 loop pause
codex1 loop resume
codex1 loop deactivate

codex1 close check
codex1 close complete
codex1 close record-review

codex1 ralph stop-hook
```

Every command:

- has help
- supports `--json`
- emits stable result/error envelopes

Mutating commands:

- require explicit `--mission` except `init` and `loop activate`
- support `--repo-root`
- support `--expect-revision` when stale writer risk exists
- use transaction helper
- never mutate artifacts before lock

## Command Semantics

### `codex1 init`

Creates:

```text
PLANS/<mission-id>/
  OUTCOME.md
  PLAN.yaml
  STATE.json
  EVENTS.jsonl
  specs/
  reviews/
```

Flags:

```text
--mission <id>
--repo-root <path>
--title <title>
--json
```

Initial `OUTCOME.md` may contain placeholders, but `outcome check` must reject
them until filled.

Initial `PLAN.yaml` may be a draft scaffold, but `plan lock` must reject it
until valid.

Initial `STATE.json.revision = 0`.

### `codex1 outcome check`

Reads `OUTCOME.md`, validates required fields, reports digest.

Does not mutate state.

Rejects empty template outcome.

### `codex1 outcome ratify`

Requires valid outcome.

Mutates:

```json
"outcome": {
  "ratified": true,
  "ratified_revision": <new revision>,
  "outcome_digest": "sha256:..."
}
```

Sets phase to `plan` if appropriate.

Re-ratification after semantic outcome edit requires explicit command and should
invalidate old plan lock unless the plan is relocked.

### `codex1 plan choose-mode`

Flags:

```text
--mode normal|graph
--reason <text>
```

Records planning mode in state as a deterministic choice. This command may be
interactive in non-JSON mode, but under `--json` it must be non-interactive.

Do not make the CLI semantically decide risk. The user/Codex provides mode and
reason; CLI records and validates enum.

### `codex1 plan choose-level`

Flags:

```text
--level small|medium|large|risky
--reason <text>
```

Records requested/effective planning level.

### `codex1 plan scaffold`

Writes or updates `PLAN.yaml` under lock.

Flags:

```text
--mode normal|graph
--level small|medium|large|risky
--from-outcome
```

It must not replace an existing non-empty plan unless `--force` or an explicit
mode-compatible update flag is present.

If it mutates state metadata, do so in same transaction.

### `codex1 plan check`

Validates `PLAN.yaml` against outcome digest and mode-specific rules.

Does not mutate state.

For graph mode, must run full graph validation:

- missing dependency
- duplicate dependency
- unknown dependency
- cycle
- invalid boundary target
- mission-close gate consistency

### `codex1 plan lock`

Requires:

- outcome ratified and fresh
- plan valid
- plan outcome_digest equals current outcome digest

Mutates:

```json
"plan": {
  "locked": true,
  "locked_revision": <new revision>,
  "plan_digest": "sha256:...",
  "outcome_digest_at_lock": "sha256:..."
}
```

Initializes `STATE.json.steps` or `STATE.json.tasks` and `STATE.json.reviews`
from plan.

For normal mode:

- initialize all steps `pending`

For graph mode:

- initialize all tasks `pending`
- initialize all review boundaries `not_started`
- set `close.requires_mission_close_review` from plan

Replan lock:

```bash
codex1 plan lock --replan --supersedes <task-or-boundary> --expect-revision N
```

Requires `state.replan.required == true` unless a force flag is explicitly added
and tested.

Rules:

- validate new `PLAN.yaml`
- do not reuse superseded task IDs for different work
- mark superseded tasks/boundaries
- append new tasks/boundaries
- refresh plan digest and outcome digest at lock
- clear `replan.required`
- preserve completed unaffected tasks when valid

### `codex1 plan graph`

Read-only graph summary:

- tasks
- dependencies
- blockers
- review boundaries
- invalid edges
- current task states

It should be a filtered/status view, not a separate truth engine.

### `codex1 plan waves`

Read-only derived waves/frontier.

Must not write waves to plan/state as editable truth.

### `codex1 task next`

Filtered view of status next action:

- normal: next pending step or in-progress finish task
- graph: ready frontier/recommended wave or in-progress finish task

Does not mutate.

### `codex1 task start`

Flags:

```text
<id>
--expect-revision N
```

Requires:

- active locked plan
- task/step is ready according to status
- not blocked by review/replan/close

Mutates status to `in_progress`, records started_revision.

### `codex1 task finish`

Flags:

```text
<id>
--proof <path>
--expect-revision N
```

Requires:

- task/step in_progress
- proof present and valid if required
- proof path inside mission root

Mutates status to `complete`, records finished_revision and proof path.

If a required review boundary targets this completed task, status should project
`run_review` before downstream dependent tasks become satisfied.

### `codex1 task status`

Read-only task/step status.

### `codex1 task packet`

Creates/prints a worker packet for Codex subagents. Does not mutate mission
truth unless explicitly recording packet creation as audit.

Packet includes:

- task id
- title/goal
- acceptance
- allowed write_paths
- proof expectations
- current outcome digest
- current plan digest
- current state revision
- warning that workers do not mutate mission truth

### `codex1 review start`

Starts a planned review boundary.

Requires:

- boundary exists
- target task(s) complete
- boundary state `not_started` or repair re-review-ready state
- current plan digest matches

Mutates boundary:

```text
not_started -> review_open
repair_done -> review_open
```

Creates a review packet or returns one in JSON.

### `codex1 review packet`

Read-only or packet-writing command for reviewer input.

Includes:

- boundary target
- files/globs
- task proof
- acceptance criteria
- locked outcome summary
- plan digest
- state revision
- reviewer instructions

### `codex1 review record`

Records raw reviewer output plus main-thread adjudication.

Flags:

```text
<boundary-id>
--raw-file <json>
--adjudication-file <json>
--expect-revision N
```

Must validate:

- raw output object
- findings array
- priorities/confidence fields if present
- adjudication references valid raw finding indexes
- every decision is known

Mutates boundary:

- no findings or no accepted blockers -> `passed`
- untriaged raw findings without adjudication -> `triage_required`
- accepted blockers -> `repair_required`
- accepted blockers with exhausted budget -> `replan_required`

Writes immutable review record under `reviews/R-####.json`.

Status blocks only after durable record/adjudication exists.

### `codex1 review repair-record`

Flags:

```text
<boundary-id>
--proof <path>
--expect-revision N
```

Requires:

- boundary state `repair_required`
- accepted_blocking_count > 0
- repair_round < max_repair_rounds
- proof valid

Mutates:

- increments `repair_round` exactly once
- records repair proof
- sets boundary to `repair_done`

Next status should project `run_review` for the targeted re-review.

### `codex1 review status`

Read-only view of boundaries, findings, repair budgets.

### `codex1 replan record`

Records that the current plan cannot be repaired within budget or that a
current accepted blocker requires changing the route while preserving the
ratified outcome.

Usually called automatically after review budget exhaustion, but CLI command
exists for explicit durable transition.

Mutates:

```json
"replan": {
  "required": true,
  "reason": "...",
  "boundary_id": "RB-T4-1",
  "supersedes": ["T4"]
}
```

### `codex1 replan check`

Read-only status of required replan and relock readiness.

### Loop Commands

`loop activate`:

- requires ratified outcome and locked fresh plan
- sets active true, paused false
- mode: `execute`, `autopilot`, or `review_loop`
- writes `ACTIVE.json`

`loop pause`:

- explicit mission required
- sets paused true
- Ralph allows stop

`loop resume`:

- explicit mission required
- sets paused false

`loop deactivate`:

- explicit mission required
- sets active false, paused false, mode none
- clears active pointer if it points to this mission

Do not let inactive loops project autonomous required work.

### `codex1 close check`

Read-only filtered view of status close readiness.

Must share code with status.

### `codex1 close record-review`

Records mission-close review outcome.

This is for terminal boundary only. Planned task review uses `review record`.

Requires:

- status next_action `close_review`
- raw review output and adjudication

Mutates:

- no accepted blockers -> `mission_close_review_passed`
- accepted blockers within budget -> repair/triage path
- exhausted budget -> replan_required

### `codex1 close complete`

Requires status next_action `close_complete`.

Under lock:

- re-read state
- re-project status
- verify close_complete still ready
- generate/overwrite `CLOSEOUT.md` for current pre-terminal revision
- compute closeout digest
- write terminal state at next revision
- clear active pointer if it points to this mission

Never terminalize stale closeout evidence.

## Doctor

`codex1 doctor --json` is install-time diagnostic, not runtime fallback.

Required checks:

- installed `codex1` command on PATH
- installed command works from outside source checkout
- `codex1 --help`
- `codex1 --json doctor`
- official Codex repo exists at `/Users/joel/.codex/.codex-official-repo`
- Stop-hook input schema includes expected fields
- Stop-hook output schema supports `decision: block` and non-empty `reason`
- hook config source supports `Stop`, `timeout`, `statusMessage`
- `codex_hooks` feature exists and is default-on in current repo snapshot
- bundled model catalog contains `gpt-5.5` and `gpt-5.4-mini`

Optional/deep checks:

- subagent hook-disable e2e proof
- parsing exact hook snippets through official config tests if practical
- official review protocol shape

`doctor.ok` must be false if required checks fail.

Warnings/info are only for optional diagnostics.

State/event drift diagnostics:

- if selected mission state revision > latest event revision -> warning
- if events missing/empty while revision > 0 -> warning
- malformed event rows -> warning
- doctor must not crash on unreadable/non-object state/events

## Official Codex Integration Facts To Verify

Verify from local official repo before coding final assertions:

```text
/Users/joel/.codex/.codex-official-repo
```

Known areas:

- Stop-hook input/output generated schemas
- Stop-hook event source
- hook config TOML parser/tests
- hook discovery/assembly code
- `codex_hooks` feature
- model catalog
- official review output protocol/prompt
- guardian/custom review session hook behavior when relevant
```

Do not cite stale paths without checking.

## Build Order

Follow this exact build order. Do not jump to commands first.

### Gate 0: Skeleton And Tooling

- create project
- choose runtime via `cli-creator`
- add `codex1` binary
- add test runner
- add formatter/linter if natural
- add `Makefile` with:
  - `make test`
  - `make install-local`
  - `make smoke`

Acceptance:

```bash
codex1 --help
codex1 --json doctor
```

may be minimal but must not crash.

### Gate 1: Types, Schemas, Errors

Implement typed structs/enums for:

- outcome
- plan
- state
- events
- review records
- proof
- closeout
- status
- errors

Implement schema validation and stable errors.

Tests first:

- non-object JSON rejected cleanly
- invalid booleans rejected
- invalid revision rejected
- unsupported schema rejected
- mission id mismatch rejected

### Gate 2: Paths And Resolver

Implement:

- repo root discovery
- `PLANS/` discovery from subdirectories even without git
- mission id validation
- active pointer validation
- Ralph cwd selection

Tests:

- absolute mission id rejected
- `..` rejected
- active pointer `..` ignored/fail-open
- running status from mission dir selects mission
- running status from subdir finds PLANS
- mismatched STATE mission id invalid

### Gate 3: Artifact Parsing And Digests

Implement:

- OUTCOME frontmatter parse/write
- PLAN YAML parse/write
- canonical digest
- proof parse
- review record parse
- closeout parse/write

Tests:

- empty outcome invalid
- placeholder outcome invalid
- semantic digest change detected
- formatting-only change stable
- PLAN graph cycle rejected
- PLAN missing dependency rejected

### Gate 4: Transaction Engine

Implement mission lock, atomic state write, event append, warnings.

Tests:

- existing lock prevents state and artifact writes
- revision conflict stable shape
- event append failure commits state and returns warning, no crash
- artifact writes happen under lock
- no mutation on failed precondition

### Gate 5: Plan/Graph/Review Validators

Implement validators before status/commands.

Tests:

- normal plan cannot contain graph depends_on
- graph plan full validation
- review boundary unknown task rejected
- mission-close required but no boundary rejected
- closeout path escape rejected

### Gate 6: Status Projector

Implement `project_status(snapshot)`.

Tests should use fixture snapshots, not command setup only.

Must cover full priority table.

Tests:

- no mission
- inactive
- paused
- invalid state fail-open
- stale outcome digest invalid_state
- stale plan digest invalid_state
- in-progress finish_task
- normal pending run_step
- graph dependency frontier run_wave
- graph review boundary blocks downstream dependency
- triage_review
- repair
- repair budget -> replan
- close_review
- close_complete
- terminal complete
- manual loop mode none does not block
- unknown next action allows stop

### Gate 7: Command Adapters

Wire commands to shared engines.

Do not implement new logic in command files if it belongs in core.

Acceptance:

- every listed command has help
- implemented commands do not return NOT_IMPLEMENTED
- future non-product helpers may return NOT_IMPLEMENTED only if not in this prompt

### Gate 8: Ralph

Implement Stop-hook adapter.

Tests:

- malformed stdin -> `{}`
- stop_hook_active true -> `{}`
- cwd inside mission resolves without flags
- active pending step blocks
- manual mode none allows
- invalid state allows
- unknown action allows
- block message includes `$interrupt` or `codex1 loop pause`

### Gate 9: Doctor

Implement fast official Codex checks.

Tests:

- doctor fails if official repo missing
- doctor fails if command on PATH stale/missing
- doctor ok false when required checks fail
- doctor does not crash on malformed mission state/events

### Gate 10: End-To-End Product Tests

Write end-to-end tests for:

Normal path:

```text
init
fill OUTCOME.md
outcome check
outcome ratify
plan choose-mode normal
plan choose-level medium
plan scaffold
fill PLAN.yaml with two steps
plan check
plan lock
loop activate --mode execute
status -> run_step S1
task start S1
task finish S1 with proof
status -> run_step S2
task start S2
task finish S2 with proof
close check -> close_complete
close complete
status -> complete
```

Graph path:

```text
init
ratify outcome
plan choose-mode graph
write graph PLAN.yaml with T1 -> T2
plan check
plan lock
loop activate
status -> ready T1 only
finish T1
status -> run_review if boundary exists, otherwise ready T2
record review passed
status -> ready T2
finish T2
mission-close review required -> close_review
record close review passed
close complete
status -> complete
```

Review/repair/replan path:

```text
finish task
run review
record accepted_blocking
status -> repair
repair-record
run targeted review
record accepted_blocking again
after budget -> replan
record/relock plan
status continues with new plan
```

## Tests For Prior Review Findings

Add regression tests for every failure below:

- event append failure commits state but returns warning, no traceback
- stale closeout cannot terminalize mission
- closeout path cannot escape mission
- graph dependencies are respected in status
- invalid graph plans cannot lock
- empty outcome cannot ratify
- plan scaffold cannot write before lock
- ordinary mutating commands require explicit mission
- revision conflict returns `REVISION_CONFLICT` and `current_revision`
- ACTIVE mission id path traversal rejected
- ACTIVE non-object ignored with warning
- STATE non-object invalid_state, no traceback
- doctor non-object STATE warning, no traceback
- missing EVENTS with revision > 0 reports drift
- malformed event rows report drift
- inactive loops project inactive before replan/close
- invalid booleans produce invalid_state and allow stop
- close readiness derived from gates, not mutable close.state alone
- Ralph uses Stop-hook cwd
- Ralph validates next-action safety before blocking
- stale outcome/plan digests invalid_state
- keyed review blockers prevent close
- mission-close review cannot be skipped
- manual loop mode none never blocks
- plan choose-mode/choose-level parse flags
- doctor ok false when required checks fail
- `.DS_Store` ignored and untracked

## Packaging And Install

Add:

```text
README.md
Makefile
.gitignore
```

`.gitignore` must include:

```text
.DS_Store
target/
dist/
__pycache__/
*.pyc
```

`make install-local` should install `codex1` into `~/.local/bin` or clearly
print the install path.

Smoke test from outside source:

```bash
cd /tmp
command -v codex1
codex1 --help
codex1 --json doctor
```

## README Requirements

Write a concise README explaining:

- what Codex1 is
- what it is not
- command surface
- JSON policy
- durable file layout
- transaction guarantees
- Ralph hook snippet
- doctor
- examples for normal, graph, review/repair/replan, close

Do not write the actual skill files.

You may include a section saying later skills will call:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

But do not create `SKILL.md`.

## Autonomy Semantics To Preserve

`$execute` later means:

```text
Continue an already locked plan through planned work, planned review boundaries,
repair/replan if required by the locked plan, and close. Stop when close is
complete, interrupted, invalid_state, or explain_and_stop.
```

`$autopilot` later means:

```text
Run clarify -> plan -> execute -> review/repair/replan/close as needed. It must
ask all necessary clarify questions and must not assume user-owned decisions.
It may open a PR only if pr_intent.open_pr was clarified/ratified.
```

`$review-loop` later means:

```text
An explicit extra review/fix loop beyond ordinary planned review boundaries.
```

These are product semantics. The CLI should support them but not implement the
skills.

## Final Verification Commands

Run all project-appropriate checks.

For Rust, likely:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
make install-local
cd /tmp && codex1 --help && codex1 --json doctor
```

Also run the e2e CLI tests.

If a check cannot run because a tool is missing, say exactly which tool is
missing and what passed instead.

## Final Response Requirements

When done, report:

- files created
- runtime chosen and why
- command surface implemented
- key invariants enforced
- tests run
- any known gaps

Do not claim graph/review/replan are deferred. They are part of this product.

Do not write skills.

# Copy-Paste Prompt Ends Here

---

## Notes For Us Before Reusing This Prompt

This prompt intentionally chooses invariant completeness over speed.

The old failed branch proved that a skeleton with broad command names is worse
than no implementation, because it lets invalid durable truth enter the system.

If this prompt is too large for a single implementation thread, split only by
invariant gates, not by product feature names:

```text
Thread 1: typed schemas, paths, artifacts, transactions, status projector
Thread 2: command adapters over the already-tested core
Thread 3: doctor, Ralph, packaging, e2e hardening
```

Do not split as:

```text
normal first, graph later
review later
replan later
```

That split is what caused half-truth product surfaces.

The safer split is:

```text
No command can admit a durable state until the engine that validates and projects
that state is complete.
```
