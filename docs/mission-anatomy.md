# Mission anatomy

A mission lives entirely under `PLANS/<mission-id>/`. There is no hidden state directory, no `.ralph/`, no side-cache. Everything the CLI or a human needs is visible in files you can `cat` and diff.

## Layout

```
PLANS/<mission-id>/
  OUTCOME.md           # clarified destination truth
  PLAN.yaml            # route, architecture, task DAG
  STATE.json           # current operational state
  EVENTS.jsonl         # append-only audit log
  specs/
    T1/
      SPEC.md          # task-local spec (static)
      PROOF.md         # task-local proof (written at task finish)
    T2/
      ...
  reviews/
    R1.md              # main-thread-recorded review findings
    R2.md
  CLOSEOUT.md          # final terminal summary (written at close complete)
```

See [`codex1-rebuild-handoff/03-planning-artifacts.md`](codex1-rebuild-handoff/03-planning-artifacts.md) for the artifact contracts.

## Files

### `OUTCOME.md`

Clarified-destination truth. YAML frontmatter plus human-readable body. Populated by `$clarify`, ratified by `codex1 outcome ratify`. A fresh mission (`codex1 init`) writes it with `[codex1-fill:*]` markers; ratification fails until every marker is resolved.

Required fields: `mission_id`, `status`, `title`, `original_user_goal`, `interpreted_destination`, `must_be_true`, `success_criteria`, `non_goals`, `constraints`, `definitions`, `quality_bar`, `proof_expectations`, `review_expectations`, `known_risks`, `resolved_questions`.

The mission cannot enter `plan` phase until `OUTCOME.md` is ratified.

### `PLAN.yaml`

The route, architecture, task DAG, planning evidence, and mission-close criteria. Written by `$plan` via `codex1 plan scaffold` and edited by the main thread until it passes `codex1 plan check`. On successful `plan check`, the plan is locked (`STATE.json` records `plan.locked = true` and `plan.hash`).

Required sections: `mission_id`, `planning_level: {requested, effective}`, `outcome_interpretation`, `architecture`, `planning_process.evidence`, `tasks`, `risks`, `mission_close.criteria`.

Waves are **not** stored here. They are derived each time `codex1 plan waves` is called, from `tasks[].depends_on` and current task status.

### `STATE.json`

Current operational state. Owned by the CLI; never edited by hand. Mutated only through `state::mutate`, which:

1. Acquires an exclusive `fs2` lock on `STATE.json.lock`.
2. Parses the current `STATE.json`.
3. Enforces `--expect-revision <N>` when provided.
4. Runs the handler closure.
5. Bumps `revision` and `events_cursor` by 1.
6. Atomically writes `STATE.json` (temp-in-same-dir + rename).
7. Appends one line to `EVENTS.jsonl`.
8. Releases the lock.

Fields (see `crates/codex1/src/state/schema.rs` for the authoritative definition): `mission_id`, `revision`, `schema_version`, `phase`, `loop`, `outcome`, `plan`, `tasks`, `reviews`, `replan`, `close`, `events_cursor`.

### `EVENTS.jsonl`

Append-only audit log. One JSON line per mutation. The `seq` of the latest line matches `state.events_cursor`. This file is audit history, not replay authority — it is not read back to reconstruct state.

### `specs/T<id>/SPEC.md`

Task-local spec: goal, allowed read/write paths, steps, acceptance criteria, proof commands, review expectations. Written during planning (`$plan` or `codex1 plan scaffold`). Static once the plan is locked — if a task's spec needs to change, replan adds a new task id.

### `specs/T<id>/PROOF.md`

Written by the worker (or main thread) when they call `codex1 task finish <id> --proof <path>`. Records commands run and their outcomes.

### `reviews/*.md`

Main-thread-recorded review findings. Reviewers never write here directly — they return findings text to the main thread, which records via `codex1 review record <id> --findings-file <path>` (or `--clean` if there were none).

### `CLOSEOUT.md`

Written by `codex1 close complete` when the mission reaches terminal. Contains the mission summary, final verdict, and links to proofs and reviews.

## Phase transitions

```
clarify        --outcome.ratified=true---------->  plan
plan           --plan.locked=true----------------> execute
execute        --all tasks Complete/Superseded--> review_loop
review_loop    --mission-close review passed----> mission_close
mission_close  --close complete------------------> terminal
```

`codex1 status` reports the current phase alongside the derived `verdict`. The mapping from state to verdict is in `state::readiness::derive_verdict` and matches `cli-contract-schemas.md`.

## Revision discipline

Every mutation bumps `state.revision` by 1. Mutating commands accept `--expect-revision <N>` for strict-equality stale-writer protection. On mismatch, the CLI returns:

```json
{
  "ok": false,
  "code": "REVISION_CONFLICT",
  "message": "Expected state revision 7 but current revision is 8.",
  "retryable": true,
  "context": { "expected": 7, "actual": 8 }
}
```

`retryable: true` on this code is a signal to callers: re-read `STATE.json`, reconcile, and retry with the current revision.

## Late-output vocabulary

Review records are classified by the CLI according to when they arrive:

- `accepted_current` — recorded before the review boundary closed. Only this category affects the consecutive-dirty counter.
- `late_same_boundary` — arrived after current, still within the same boundary revision.
- `stale_superseded` — belongs to a superseded task/review boundary.
- `contaminated_after_terminal` — arrived after mission terminal.

All four categories are appended to `EVENTS.jsonl`; only `accepted_current` mutates current truth. This is how the harness tolerates subagents that respond after their boundary has moved on.

## Stop semantics

Ralph, the stop hook, reads `codex1 status --json` and nothing else. `stop.allow` is true iff the loop is inactive / paused, or the verdict is in `{terminal_complete, mission_close_review_passed, needs_user}`. A mission can be paused for discussion via `$close` (`codex1 loop pause`), resumed via `codex1 loop resume`, and abandoned via `codex1 loop deactivate`.

See [`cli-contract-schemas.md`](cli-contract-schemas.md) for the full verdict/`stop.allow` derivation rules and [`codex1-rebuild-handoff/01-product-flow.md`](codex1-rebuild-handoff/01-product-flow.md) for the user-facing workflow that these files drive.
