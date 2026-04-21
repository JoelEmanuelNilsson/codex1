# CLI Contract — Schemas Reference

This file is the authoritative schema reference for the `codex1` CLI. Phase B workers use it as their contract. Foundation writes it once; Phase B does **not** modify it.

If you are a Phase B worker:

- Use the envelope shapes below for every command's `--json` output.
- Use only the error codes in this file. Do not invent new codes.
- Do not mutate any file or type defined as "Foundation-owned" below.
- Preserve the subcommand enum variants declared in `crates/codex1/src/cli/*/mod.rs`. You may add sibling modules under your directory, and you may replace the `dispatch` function body, but keep the enum variants stable so `cli::mod` continues to compile.

## JSON envelopes

### Success
```json
{
  "ok": true,
  "mission_id": "demo",        // optional; present on mission-bound commands
  "revision": 7,               // optional; present after a mutation or state read
  "data": { /* command-specific */ }
}
```

### Error
```json
{
  "ok": false,
  "code": "PLAN_INVALID",
  "message": "Task T3 is missing depends_on.",
  "hint": "Add depends_on: [] for root tasks or depends_on: [T…] for dependent tasks.",
  "retryable": false,
  "context": { /* free-form, command-specific */ }
}
```

**Optional fields.** `hint` and `context` are omitted from the serialized
envelope when they are null / empty — the CLI uses
`#[serde(skip_serializing_if = "…")]` for both. Callers MUST treat
missing `hint`/`context` as equivalent to the empty value. `ok`, `code`,
`message`, and `retryable` are always present.

## Error codes (canonical set)

| Code | Meaning | Retryable |
| --- | --- | --- |
| `OUTCOME_INCOMPLETE` | Required OUTCOME.md field missing, empty, or placeholder. | No |
| `OUTCOME_NOT_RATIFIED` | Command needs a ratified OUTCOME.md. | No |
| `PLAN_INVALID` | PLAN.yaml structure or task shape fails validation. | No |
| `DAG_CYCLE` | Cycle detected in the task DAG. | No |
| `DAG_MISSING_DEP` | `depends_on` references an unknown task id. | No |
| `TASK_NOT_READY` | Task's dependencies or state forbid this transition. | No |
| `PROOF_MISSING` | `task finish` called without a readable proof file. | No |
| `REVIEW_FINDINGS_BLOCK` | Active P0/P1/P2 findings block progress. | No |
| `REPLAN_REQUIRED` | Six consecutive dirty reviews on same active target. | No |
| `CLOSE_NOT_READY` | `close complete` called before `close check` passes. | No |
| `STATE_CORRUPT` | STATE.json missing or unparseable. | No |
| `REVISION_CONFLICT` | `--expect-revision N` mismatched the on-disk revision. | **Yes** |
| `STALE_REVIEW_RECORD` | Review record references a superseded boundary. | No |
| `TERMINAL_ALREADY_COMPLETE` | Mission already closed. | No |
| `CONFIG_MISSING` | Required config file absent. | No |
| `MISSION_NOT_FOUND` | Could not resolve a mission directory. | No |
| `PARSE_ERROR` | IO, JSON, or YAML parsing failure. | No |
| `NOT_IMPLEMENTED` | Reserved legacy code for unavailable command implementations. | No |

## Global flags (all commands)

| Flag | Purpose |
| --- | --- |
| `--help` | Clap-generated help text. |
| `--json` | Reserved; JSON output is the default. Present for cli-creator parity. |
| `--mission <id>` | Directory name under PLANS/. Optional for `doctor`, `hook snippet`. |
| `--repo-root <path>` | Overrides CWD discovery. |
| `--dry-run` | Mutating commands only. Validates and reports without writing. |
| `--expect-revision <N>` | Mutating commands only. Strict equality; returns `REVISION_CONFLICT`. |

## Mission resolution precedence

1. `--mission <id>` + `--repo-root <path>` → `<path>/PLANS/<id>/`.
2. `--mission <id>` alone → `<CWD>/PLANS/<id>/`.
3. Neither → walk up from CWD to the nearest single-mission `PLANS/` directory. Error if 0 or >1 candidates.
4. Paths resolve to absolute; symlinks are followed by the filesystem.

For `codex1 status` specifically, the graceful `foundation_only` fallback applies only to the true bare “no mission anywhere” case. Ambiguous multi-mission discovery or an explicit bad `--repo-root` / `--mission` still return `MISSION_NOT_FOUND`.

### Per-command relative path resolution

Some commands take path arguments that reference artifacts inside the
mission directory. The resolution rule is pinned here so callers don't
have to discover it by trial:

- `codex1 task finish T<id> --proof <path>`: relative paths are resolved
  against the mission directory (e.g. `--proof specs/T1/PROOF.md`
  resolves to `<mission_dir>/specs/T1/PROOF.md`). Absolute paths are
  used verbatim. Relative proof paths must stay inside the mission tree;
  mission-local symlink escapes are refused. Rationale: skills and workers
  think in terms of the mission, not the CWD.
- `codex1 review record T<id> --findings-file <path>`: resolved against
  CWD. The findings file is the caller's own output, not a mission
  artifact; the CLI copies it into `reviews/<id>.md` under the mission.
- `codex1 close record-review --findings-file <path>`: same as above —
  CWD-relative; copied to `reviews/mission-close-<rev>.md`.

## STATE.json schema

Defined in `crates/codex1/src/state/schema.rs`. Pinned shape (Rust types):

```rust
struct MissionState {
  mission_id: String,
  revision: u64,              // bumps +1 per successful mutation
  schema_version: u32,        // 1
  phase: Phase,               // clarify|plan|execute|review_loop|mission_close|terminal
  loop: LoopState,            // { active, paused, mode }
  outcome: OutcomeState,      // { ratified, ratified_at }
  plan: PlanState,            // { locked, requested_level, effective_level, hash }
  tasks: BTreeMap<TaskId, TaskRecord>,
  reviews: BTreeMap<TaskId, ReviewRecord>,
  replan: ReplanState,        // { consecutive_dirty_by_target, triggered, triggered_reason }
  close: CloseState,          // { review_state, terminal_at }
  events_cursor: u64,
}
```

Downstream units mutate only the fields they own:

- `cli-outcome` → `outcome`, `phase`.
- `cli-plan-*` → `plan`, `phase`.
- `cli-task` → `tasks`, `phase`.
- `cli-review` → `reviews`, `replan.consecutive_dirty_by_target`,
  `tasks[review_task_id]` (review record transitions the review task
  itself to Complete on clean, matching normal task lifecycle).
- `cli-replan` → `replan`.
- `cli-loop` → `loop`.
- `cli-close` → `close`, `phase`,
  `replan.consecutive_dirty_by_target["__mission_close__"]`.

## Mutation protocol

```rust
state::mutate(&paths, expected_revision, "kind", payload_json, |state| {
    // Adjust fields in place. Return Err to abort without writing.
    Ok(())
})
```

The helper:
1. Acquires an exclusive fs2 lock on `STATE.json.lock`.
2. Reads and parses `STATE.json`.
3. If `expected_revision` is `Some(n)`, returns `REVISION_CONFLICT` on mismatch.
4. Calls the closure.
5. Bumps `revision` and `events_cursor` by 1.
6. Appends one line to `EVENTS.jsonl`.
7. Atomically writes `STATE.json` (temp-in-same-dir + rename).
8. Releases the lock.

Do not write `STATE.json` directly from a handler. Always go through `state::mutate` (or `state::init_write` for `codex1 init`).

## Review record freshness

Review records are classified by the CLI into:

- `accepted_current` — recorded before the review boundary closed.
- `late_same_boundary` — arrived after current but within the same boundary revision.
- `stale_superseded` — belongs to a superseded task/review boundary.
- `contaminated_after_terminal` — arrived after mission terminal.

Only `accepted_current` affects the consecutive-dirty counter. Others are appended to EVENTS.jsonl for audit.

## Dirty counter rules

- Per active review target.
- `accepted_current` **dirty** → increment by 1.
- `accepted_current` **clean** → reset to 0.
- `late_same_boundary` / `stale_superseded` / `contaminated_after_terminal` → do not affect counter.
- Reset to 0 on replan (`replan record`).
- Counter value ≥ 6 sets `replan.triggered = true` on the successful
  `review record` mutation; later readiness surfaces report
  `REPLAN_REQUIRED`.

## Verdict derivation (shared by `status` and `close check`)

Implemented in `crates/codex1/src/state/readiness.rs`. Foundation-owned; do not duplicate.

```
if close.terminal_at.is_some()      -> terminal_complete
if !outcome.ratified                -> needs_user
if !plan.locked                     -> needs_user
if replan.triggered                 -> blocked
if any review has Dirty verdict     -> blocked
if all tasks complete/superseded:
    review_state == NotStarted      -> ready_for_mission_close_review
    review_state == Open            -> mission_close_review_open
    review_state == Passed          -> mission_close_review_passed
else                                -> continue_required
```

`close_ready = (verdict == mission_close_review_passed) && no close-path blockers remain`.

`stop.allow` is true iff the loop is inactive/paused, or the verdict is in `{terminal_complete, mission_close_review_passed, needs_user}`.

## Per-command data shapes (Phase B contract)

### `outcome check`
```json
{ "ok": true, "data": { "ratifiable": true, "missing_fields": [], "placeholders": [] } }
```
On failure: `OUTCOME_INCOMPLETE` with `context.missing_fields` and `context.placeholders`.

### `outcome ratify`
Success: `{ "data": { "ratified_at": "2026-04-20T…Z" } }`. Fails with `OUTCOME_INCOMPLETE` if check fails.

### `plan choose-level`
```json
{ "data": { "requested_level": "medium", "effective_level": "hard", "escalation_reason": "…",
            "next_action": { "kind": "plan_scaffold", "args": ["codex1","plan","scaffold","--level","hard"] } } }
```

### `plan scaffold`
Success: `{ "data": { "wrote": "PLANS/demo/PLAN.yaml", "specs_created": [] } }`.

### `plan check`
Success: `{ "data": { "tasks": 4, "review_tasks": 1, "hard_evidence": 3 } }`. Errors: `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`.

### `plan waves`
```json
{ "data": {
    "waves": [
      { "wave_id": "W1", "tasks": ["T1"], "parallel_safe": true, "blockers": [] },
      { "wave_id": "W2", "tasks": ["T2","T3"], "parallel_safe": true, "blockers": [] }
    ],
    "current_ready_wave": "W1",
    "all_tasks_complete": false
  } }
```

### `plan graph --format mermaid`
Success: `{ "data": { "mermaid": "flowchart TD …" } }` (and writes file with `--out`).

### `task next`
```json
{ "data": {
    "next": { "kind": "run_wave", "wave_id": "W1", "tasks": ["T1"], "parallel_safe": true }
  } }
```
Alternate shapes for review/close/replan ready states.

### `task packet <id>`
```json
{ "data": {
    "task_id": "T3",
    "title": "…",
    "spec_excerpt": "…",
    "write_paths": ["src/cli/outcome/**"],
    "proof_commands": ["cargo test outcome"],
    "mission_summary": "…"
  } }
```

### `review packet <id>`
```json
{ "data": {
    "task_id": "T4",
    "review_profile": "code_bug_correctness",
    "profiles": ["code_bug_correctness","integration_intent"],
    "targets": ["T2"],
    "target_specs": [{"task_id":"T2","spec_path":"specs/T2/SPEC.md","spec_excerpt":"…"}],
    "diffs": [{"path":"crates/codex1/src/cli/task/**"}],
    "proofs": ["PLANS/demo/specs/T2/PROOF.md"],
    "mission_summary": "…",
    "mission_id": "demo",
    "reviewer_instructions": "You are a Codex1 reviewer. …"
  } }
```
`target_specs`, `profiles`, `mission_id`, and `reviewer_instructions`
are additive convenience fields. `proofs` is the canonical name for the
list of target proof paths (the binary historically emitted
`target_proofs`; it now emits `proofs`). For in-repo proof files the
paths are reported relative to the repo root, e.g.
`PLANS/<mission>/specs/T2/PROOF.md`; externally recorded absolute proof
paths remain absolute.

### `review record --clean|--findings-file`
Success:
```json
{ "data": {
    "review_task_id": "T4",
    "verdict": "clean",
    "category": "accepted_current",
    "reviewers": ["code-reviewer","intent-reviewer"],
    "findings_file": null,
    "replan_triggered": false,
    "warnings": []
  } }
```
`findings_file` (relative path under `reviews/`), `replan_triggered`
(true when the 6-consecutive-dirty threshold was just breached), and
`warnings` (non-fatal notes such as `late_same_boundary`) are additive
fields; `review_task_id`, `verdict`, `category`, and `reviewers` are
guaranteed.

### `replan check`
```json
{ "data": { "required": false, "reason": null, "consecutive_dirty_by_target": { "T4": 2 } } }
```

### `close check`
```json
{ "data": {
    "ready": false,
    "verdict": "continue_required",
    "blockers": [
      { "code": "TASK_NOT_READY", "detail": "T7 is pending" }
    ]
  } }
```

### `close complete`
Success:
```json
{ "data": {
    "closeout_path": "PLANS/demo/CLOSEOUT.md",
    "terminal_at": "2026-04-20T…Z",
    "mission_id": "demo"
  } }
```
Recovery exception: if the mission is already terminal but `CLOSEOUT.md`
is missing, `close complete` repairs the missing artifact and returns
success. Otherwise subsequent calls return `TERMINAL_ALREADY_COMPLETE`.

### `close record-review --clean|--findings-file <path>`

Records the main-thread outcome of the mission-close review. Mutates
`state.close.review_state` (`NotStarted → Open` on first call, `Open →
Passed` on `--clean`) and the
`replan.consecutive_dirty_by_target["__mission_close__"]` counter. The
6-consecutive-dirty threshold still triggers a replan. Success envelope:
```json
{ "data": {
    "verdict": "clean",
    "review_state": "passed",
    "consecutive_dirty": 0,
    "findings_file": null,
    "reviewers": ["mission-close-auditor"],
    "replan_triggered": false,
    "dry_run": false
  } }
```

### `loop activate --mode <clarify|plan|execute|review_loop|mission_close>`

Sets `state.loop.active = true, paused = false, mode = <mode>`. Idempotent:
if the loop is already active in the same mode, returns `noop: true`
without a revision bump. Rejects `none` (`deactivate` exists for that)
and unknown mode strings. Same success-shape as `pause`/`resume`/
`deactivate`: the flattened `LoopState` plus `noop`, `dry_run`, optional
`before`.

### `status` (unified)
```json
{ "data": {
    "phase": "execute",
    "verdict": "continue_required",
    "loop": { "active": true, "paused": false, "mode": "execute" },
    "next_action": { "kind": "run_wave", "wave_id": "W2", "tasks": ["T2","T3"] },
    "ready_tasks": ["T2","T3"],
    "parallel_safe": true,
    "parallel_blockers": [],
    "review_required": [],
    "replan_required": false,
    "close_ready": false,
    "stop": {
      "allow": false,
      "reason": "active_loop",
      "message": "Run wave W2 or use $close to pause."
    }
  } }
```

## Foundation-owned files (Phase B: DO NOT MODIFY)

```
Cargo.toml
crates/codex1/Cargo.toml
crates/codex1/src/bin/**
crates/codex1/src/lib.rs
crates/codex1/src/cli/mod.rs
crates/codex1/src/cli/init.rs
crates/codex1/src/cli/doctor.rs
crates/codex1/src/cli/hook.rs
crates/codex1/src/core/**
crates/codex1/src/state/**        (excluding any new files Phase B adds under its own domain)
Makefile
README.md     (Phase B Unit 13 may append, but not delete Foundation sections)
docs/cli-contract-schemas.md
```

Phase B units may freely:
- Modify their own `crates/codex1/src/cli/<module>/**`.
- Add their own `crates/codex1/src/<module>/**` (logic outside the CLI layer).
- Add their own `crates/codex1/tests/<module>.rs`.
- Add their own `.codex/skills/<skill>/**`.
- Add their own `scripts/<script>` (Unit 12 ralph-hook, Unit 13 install-docs).
- Add their own `docs/<doc>.md` (Unit 13 install-docs).

If a Phase B unit needs a Foundation-level change, open a separate coordination PR instead of sneaking it into a feature PR.
