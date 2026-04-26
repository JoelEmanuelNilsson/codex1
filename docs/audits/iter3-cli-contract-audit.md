# CLI Contract Audit — iter 3

Branch audited: `main` @ `958d2f1`
Audited on: 2026-04-20 UTC
Binary: `target/release/codex1 0.1.0` (built from `958d2f1`).

## Scope

Fresh pass on every command listed in `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface. Re-ran the iter 2 checks (command surface, envelope, error codes, status ↔ close-check agreement) plus the iter 3 mandate to verify commit `958d2f1`'s F8 fix:

- `plan.task_ids` field presence on `state::schema::PlanState`.
- `plan check` writes `plan.task_ids` from the parsed PLAN.yaml DAG.
- `readiness::tasks_complete` consults `state.plan.task_ids`, not `state.tasks.values()`.
- No alternative "all tasks done" predicate anywhere else in the CLI bypasses the fix.
- Every caller of `tasks_complete` is `derive_verdict`.

## Summary

**FAIL — 2 P0, 1 P1, 1 P2.**

The iter 2 F8 fix (verdict path) correctly resolves the core semantic bug in `state::readiness::derive_verdict`. But the same commit introduced two build-evidence regressions (clippy-breaking line, test fixtures not updated) that the task's "Build evidence (required)" block treats as P0. A second, distinct `tasks_all_complete` predicate in `close/check.rs` still uses the buggy pattern and produces misleading blocker messages; this is the audit-scope question "is there an alternative `all tasks done` predicate anywhere else" answered with "yes." A latent migration hazard affects any `STATE.json` locked by a pre-`958d2f1` binary.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | **FAIL** (see F1) |
| `cargo test --release --no-fail-fast` | **FAIL** — 167 passed / 2 failed / 169 total (see F2) |

Verbatim clippy failure:

```text
error: assigning the result of `Clone::clone()` may be inefficient
   --> crates/codex1/src/cli/plan/check.rs:109:13
    |
109 |             s.plan.task_ids = task_ids_to_record.clone();
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: use `clone_from()`: `s.plan.task_ids.clone_from(&task_ids_to_record)`
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.94.0/index.html#assigning_clones
    = note: `-D clippy::assigning-clones` implied by `-D warnings`
error: could not compile `codex1` (lib) due to 1 previous error
```

Verbatim test failures:

```text
test all_tasks_complete_reports_ready_for_mission_close_review ... FAILED
test mission_close_review_passed_reports_close_next_action ... FAILED
test result: FAILED. 12 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

## Findings

### F1 (P0): clippy regression in the F8 fix itself

- **Severity:** P0. The task's "Build evidence (required)" block explicitly requires `cargo clippy --all-targets -- -D warnings` to pass. It does not. The regression was introduced by the same commit (`958d2f1`) that was supposed to close iter 2's P0.
- **Where:** `crates/codex1/src/cli/plan/check.rs:109`:
  ```rust
  s.plan.task_ids = task_ids_to_record.clone();
  ```
- **Lint:** `clippy::assigning_clones` — stable in Rust 1.78, enabled here by `-D warnings`.
- **Expected:** `s.plan.task_ids.clone_from(&task_ids_to_record);` (per clippy's own fix hint).
- **Fix sketch:** one-line replacement; the closure already holds a `&task_ids_to_record` capture, so `clone_from` works as-is. Alternatively move the value (the closure is a `FnOnce`, so `s.plan.task_ids = task_ids_to_record;` would compile after restructuring — but `clone_from` is the cleanest drop-in.)
- **Evidence of provenance:** `git show 958d2f1 -- crates/codex1/src/cli/plan/check.rs` shows line 109 is part of this commit; iter 1 commit `271b2fc` touched a different file; `6473650` did not touch `plan/check.rs`. The regression is unique to iter 2's F8 fix.

### F2 (P0): test regressions in the F8 fix

- **Severity:** P0. `cargo test --release` is one of three required build-evidence commands. It fails on two tests.
- **Where:**
  - `crates/codex1/tests/status.rs:329` — `all_tasks_complete_reports_ready_for_mission_close_review`
  - `crates/codex1/tests/status.rs:361` — `mission_close_review_passed_reports_close_next_action`
- **Root cause:** Both fixtures set `plan.locked = true` and insert `T1..T4` into `state.tasks` with status `Complete`, but never populate `state.plan.task_ids`. After the F8 fix, `readiness::tasks_complete` consults `state.plan.task_ids` (empty → `return false`), so the verdict collapses to `continue_required` instead of `ready_for_mission_close_review` / `mission_close_review_passed`.
- **Failing assertions:**
  ```text
  assertion `left == right` failed
    left: String("continue_required")
   right: "ready_for_mission_close_review"
  (at crates/codex1/tests/status.rs:355)

  assertion `left == right` failed
    left: String("continue_required")
   right: "mission_close_review_passed"
  (at crates/codex1/tests/status.rs:388)
  ```
- **Why this is a real regression, not a stale test:** the F8 commit's diff (`git show 958d2f1 --stat`) updated fixtures in `crates/codex1/tests/close.rs` (via `StateBuilder::build()` that now auto-populates `task_ids`) and `crates/codex1/tests/status_close_agreement.rs` (explicit `s.plan.task_ids = dag.clone()` at every `plan.locked = true` site). The same pass failed to update `crates/codex1/tests/status.rs`, which hand-builds fixtures in-line without the builder. Both `status.rs` tests express valid contract expectations: the first says "all DAG tasks complete + close.NotStarted → `ready_for_mission_close_review`"; the second says "all DAG tasks complete + close.Passed → `mission_close_review_passed` + `close_ready: true`". Neither expectation changed; only the fixtures need the new `task_ids` field.
- **Fix sketch (not applied — audit-only):** Add `state.plan.task_ids = vec!["T1".into(), "T2".into(), "T3".into(), "T4".into()];` alongside the `state.plan.locked = true` line in each of the two tests (lines 333 and 365). That matches the pattern the fix applied in `status_close_agreement.rs`.
- **Scope note:** `status.rs` has 10 total sites that set `plan.locked = true`; only these two additionally require `tasks_complete == true` to satisfy their assertions. The other 8 tests correctly pin `continue_required` regardless of `task_ids`, so the regression is bounded to these two.

### F3 (P1): alternative "all tasks done" predicate still uses the buggy pattern

- **Severity:** P1. The audit scope explicitly asked: "Verify there is no alternative `all tasks done` predicate anywhere else in the CLI that could bypass the fix." There is one.
- **Where:** `crates/codex1/src/cli/close/check.rs:132-140`:
  ```rust
  fn tasks_all_complete(state: &MissionState) -> bool {
      if state.tasks.is_empty() {
          return false;
      }
      state
          .tasks
          .values()
          .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded))
  }
  ```
  Used at `close/check.rs:111` to gate the `CLOSE_NOT_READY` blocker emission. This is the exact pattern that iter 2 rewrote in `state::readiness::tasks_complete`, untouched.
- **Observed live impact** (reproduction recorded below):

  Setup: `plan.locked=true`, `plan.task_ids=["T1","T2","T3","T4"]`, `state.tasks={T1: Complete}`, `close.review_state = NotStarted`, full PLAN.yaml present.
  ```bash
  $ codex1 --json close check --mission demo
  {
    "ok": true,
    "data": {
      "blockers": [
        { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" }
      ],
      "ready": false,
      "verdict": "continue_required"
    }
  }
  ```
  `verdict: continue_required` is correct (via the fixed `readiness::tasks_complete`). But the blocker list says "mission-close review has not started" — which is a false signal. The real blockers are T2, T3, T4 being un-started. The `for (task_id, record) in &state.tasks` loop at `check.rs:95` only iterates entries that exist in `state.tasks`; T2/T3/T4 are not in the map, so no `TASK_NOT_READY` blocker is emitted for them. Then `tasks_all_complete` looks only at `state.tasks.values()` (just T1: Complete) and returns true, so the code falls into the `match state.close.review_state` arm and emits `CLOSE_NOT_READY` as if mission-close review were the problem.

  The same misleading detail reaches `close complete` via `ReadinessReport::blocker_summary()` (used at `cli/close/complete.rs:44`):
  ```bash
  $ codex1 --json close complete --mission demo
  {
    "ok": false,
    "code": "CLOSE_NOT_READY",
    "message": "Mission is not ready for close: CLOSE_NOT_READY: mission-close review has not started"
  }
  ```
  The gate itself still holds (`ready = matches!(verdict, Verdict::MissionCloseReviewPassed)` at `check.rs:49`), so `close complete` still correctly refuses. The user just sees the wrong reason.
- **Why P1, not P0:** `close complete`'s refusal gate (via `ready`) derives from the fixed `derive_verdict` path, so the bug does not allow a bad close to proceed. Impact is confined to misleading blocker messages — a user-facing correctness bug, not a contract break.
- **Expected:** Route `tasks_all_complete` through the same DAG snapshot (`state.plan.task_ids`) as `readiness::tasks_complete`, or even better — delete the duplicate predicate and have `close/check.rs` use `readiness::tasks_complete` directly.
- **Fix sketch (not applied):**
  ```rust
  fn tasks_all_complete(state: &MissionState) -> bool {
      let dag = &state.plan.task_ids;
      if dag.is_empty() {
          return false;
      }
      dag.iter().all(|id| {
          state
              .tasks
              .get(id)
              .is_some_and(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded))
      })
  }
  ```
  And also extend the blocker loop at `check.rs:95` to emit `TASK_NOT_READY` for any DAG node missing from `state.tasks`. Reference: the fixed `readiness::tasks_complete` at `state/readiness.rs:74-93`.
- **Contract reference:** `docs/cli-contract-schemas.md:299-307` pins the `close check` blocker shape. The shape is honored — the codes are canonical — but the concrete content misleads.

### F4 (P2): locked STATE.json from pre-`958d2f1` binaries cannot reach mission-close readiness under the new binary

- **Severity:** P2. No shipping code is affected (project is pre-v1; all test fixtures have been migrated). But any `STATE.json` file locked by a binary built before commit `958d2f1` will deserialize with `plan.task_ids = Vec::new()` (via `#[serde(default)]` at `state/schema.rs:84`), and the `plan check` idempotent short-circuit at `cli/plan/check.rs:62-84` prevents a backfill.
- **Reproduction:**
  - Seed a STATE.json with `plan.locked=true`, `plan.hash="sha256:<real>"`, no `task_ids`, and every DAG task complete (`state.tasks = {T1:Complete, T2:Complete, T3:Complete, T4:Complete}`).
  - Verified live: `status --json --mission demo` → `verdict: continue_required`, `close_ready: false`, `close check` → `CLOSE_NOT_READY` blocker.
  - Running `plan check` on the unchanged PLAN.yaml would hit the idempotent short-circuit (same hash + already locked) and would not rewrite `task_ids`. The mission would stay stuck until the user either edits PLAN.yaml (to force a different hash, re-triggering the mutation closure which will then write `task_ids`), or the user hand-edits STATE.json (forbidden by the handoff). This branch is established by code inspection of `cli/plan/check.rs:62-84`; my live smoke test uses a synthetic hash and so does not itself replay the short-circuit path.
- **Why P2, not P1:** There are no pre-`958d2f1` locked missions in flight — the project has no installed base. But the handoff's doc surface does not warn about the migration. A future packager or early user upgrading across this commit will hit a locked state with no clean CLI recourse.
- **Fix sketch (not applied):** Two compatible options:
  1. In `cli/plan/check.rs:62-84`, relax the idempotent short-circuit when `current.plan.task_ids.is_empty()` — i.e. still run the mutation (just to backfill the missing field) even on a hash match. The revision will bump by 1 but the closure idempotently writes the same non-`task_ids` fields, so the mission remains consistent.
  2. Add a `codex1 plan migrate` one-shot that rewrites `task_ids` from PLAN.yaml without requiring a hash change. Less surgical than option 1.
  Option 1 is strongly preferred because it makes the migration invisible: every legacy mission that runs `plan check` once under the new binary heals itself.
- **Contract reference:** `docs/cli-contract-schemas.md:134-153` (mutation protocol) and `docs/codex1-rebuild-handoff/03-planning-artifacts.md` (plan-lock semantics) neither mention this migration step.

## F8-specific verification (commit `958d2f1`)

### `plan.task_ids` field presence

`crates/codex1/src/state/schema.rs:71-86`:
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanState {
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_level: Option<PlanLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_level: Option<PlanLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default)]
    pub task_ids: Vec<TaskId>,
}
```
Field is present, defaults to empty Vec when absent in an existing `STATE.json` (the `#[serde(default)]` makes old files forward-compatible but also enables F4).

### `plan check` writes `plan.task_ids`

`crates/codex1/src/cli/plan/check.rs:95-114`:
```rust
let task_ids_to_record = summary.task_ids.clone();
let mutation = state::mutate(
    &paths,
    ctx.expect_revision,
    "plan.checked",
    event_payload,
    |s| {
        s.plan.locked = true;
        s.plan.hash = Some(hash.clone());
        s.plan.requested_level = Some(summary.requested_level.clone());
        s.plan.effective_level = Some(summary.effective_level.clone());
        s.plan.task_ids = task_ids_to_record.clone();    // ← F8 fix (also the site of F1 clippy regression)
        if matches!(s.phase, Phase::Plan) {
            s.phase = Phase::Execute;
        }
        Ok(())
    },
)?;
```
`Summary::task_ids` is computed by `validate_tasks` at `check.rs:240,316-342` and returned into `Summary` at `check.rs:312`. The IDs are collected in plan order from every `tasks[].id` pass.

Caveat: the mutation closure is entered only on the non-short-circuit path. Idempotent re-runs (`already_locked_same`, `check.rs:62-84`) do not enter it — this is the root of F4.

### `readiness::tasks_complete` consults `state.plan.task_ids`

`crates/codex1/src/state/readiness.rs:74-93`:
```rust
fn tasks_complete(state: &MissionState) -> bool {
    let dag = &state.plan.task_ids;
    if dag.is_empty() {
        return false;
    }
    dag.iter().all(|id| {
        state
            .tasks
            .get(id)
            .is_some_and(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded))
    })
}
```
Correct — iterates the DAG snapshot, requires every node to have an entry in `state.tasks` with a terminal status. `state::mutate` never writes partial DAG views; `plan.task_ids` is written once, atomically, inside the `plan.checked` mutation closure.

### Callers of `tasks_complete`

Grep over the workspace for `tasks_complete` (the fixed helper):

| Site | Kind |
| --- | --- |
| `crates/codex1/src/state/readiness.rs:74` | Definition |
| `crates/codex1/src/state/readiness.rs:57` | Sole call site — `derive_verdict` |
| `crates/codex1/src/state/schema.rs:82` | Doc comment ("`readiness::tasks_complete` can recognize …") |
| `crates/codex1/tests/status_close_agreement.rs:162` | Test comment |
| `crates/codex1/tests/status_close_agreement.rs:373` | Test comment |

One caller. The fix's chokepoint (`derive_verdict` → `tasks_complete`) is honored.

### Alternative "all tasks done" predicates in the CLI

Grep over the workspace for `all_tasks_complete | all_tasks_done | tasks\.values\(\)\.all`:

| Site | Status |
| --- | --- |
| `crates/codex1/src/cli/plan/waves.rs:89-98` | **Safe.** `let all_tasks_complete = !tasks.is_empty() && tasks.iter().all(…)` — `tasks` here is the parsed `ParsedTask` list loaded from PLAN.yaml (`load_plan_tasks` at `waves.rs:45-58`), NOT the on-disk state map. Structurally equivalent to the DAG snapshot; no bypass. |
| `crates/codex1/src/cli/close/check.rs:132-140` | **F3 finding** — duplicate of the buggy pattern, still live. |
| Tests in `tests/plan_waves.rs`, `tests/e2e_full_mission.rs`, etc. | Assertion-only; read the `all_tasks_complete` field emitted by `plan waves`. Not predicates. |

There is exactly one alternative predicate (F3), and it is the one iter 2 missed.

## Clean checks (iter 2 re-run)

### Minimal command surface (28 verbs per `02-cli-contract.md` § Minimal Command Surface)

All verbs exist in the clap dispatch tree and expose `--help`. The surface now formally includes `loop activate` and `close record-review` (iter 1 fix for baseline P1-2). Verified live against the compiled binary.

| Verb | Source |
| --- | --- |
| `init`, `status`, `doctor`, `hook snippet` | `cli/mod.rs:76-125` |
| `outcome check`, `outcome ratify` | `cli/outcome/mod.rs` |
| `plan choose-level`, `plan scaffold`, `plan check`, `plan graph`, `plan waves` | `cli/plan/mod.rs:22-67` |
| `task next`, `task start`, `task finish`, `task status`, `task packet` | `cli/task/mod.rs` |
| `review start`, `review packet`, `review record`, `review status` | `cli/review/mod.rs` |
| `replan check`, `replan record` | `cli/replan/mod.rs` |
| `loop activate`, `loop pause`, `loop resume`, `loop deactivate` | `cli/loop_/mod.rs:24-47` |
| `close check`, `close complete`, `close record-review` | `cli/close/mod.rs:43-61` |

### Stable JSON envelopes

- Success envelope: `{ ok: true, mission_id?, revision?, data }` per `core/envelope.rs:20-29`. Verified live on `doctor`, `init`, `status`, `hook snippet`, `outcome check`, `plan choose-level`, `plan waves`, `plan graph`, `task next`, `replan check`.
- Error envelope: `{ ok: false, code, message, hint?, retryable, context? }` per `core/envelope.rs:67-78`. `hint` and `context` are optional-when-null (documented now at `docs/cli-contract-schemas.md:36-40`, iter 1 fix for baseline P2-2).

### Error codes vs canonical `CliError` set

- `CliError::Other / INTERNAL` is gone — iter 1 fix for baseline P2-1. `crates/codex1/src/core/error.rs` defines 20 variants, every one is a canonical string in `docs/cli-contract-schemas.md:42-63`. No `anyhow::Error` propagation remains.
- Spot-checked raw-string code sites: `cli/plan/check.rs` `exit_with_validation_error` uses only `PLAN_INVALID` / `DAG_CYCLE` / `DAG_MISSING_DEP`. `cli/close/check.rs` `Blocker::new` uses only `OUTCOME_NOT_RATIFIED` / `PLAN_INVALID` / `REPLAN_REQUIRED` / `TASK_NOT_READY` / `REVIEW_FINDINGS_BLOCK` / `CLOSE_NOT_READY`. All canonical.

### `plan choose-level` rejects unratified outcomes

Verified live (iter 1 fix for baseline P1-1):
```bash
$ codex1 init --mission demo
$ codex1 plan choose-level --level medium --mission demo
{ "ok": false, "code": "OUTCOME_NOT_RATIFIED", "message": "OUTCOME.md is not ratified", "retryable": false }
```
Implemented at `cli/plan/choose_level.rs:25-28` (state load + early return).

### `status` ↔ `close check` verdict agreement

`state::readiness::derive_verdict` is the single source of truth for both (called from `cli/status/project.rs:19` and `cli/close/check.rs:47`). Seven hand-built STATE.json fixtures spanning needs_user, continue_required, blocked, ready_for_mission_close_review, mission_close_review_open, mission_close_review_passed, and terminal_complete all agree on `verdict` and `close_ready` / `ready`. The `close check` blocker *list* can misinform in the F3 scenario, but the verdict string itself does not disagree.

Dedicated agreement property test at `tests/close.rs:726-773` (24 states) and `tests/status_close_agreement.rs:392-420` (21 fixtures, including the `one_task_done_out_of_four` regression fixture added alongside the F8 fix) remain passing.

### `stop.allow` projection consistency

- Paused → `allow: true, reason: "paused"`
- Terminal → `allow: true, reason: "terminal"`
- Loop inactive → `allow: true, reason: "idle"`
- Active-unpaused + verdict allows stop → `allow: true, reason: "idle"`
- Active-unpaused + verdict doesn't allow stop → `allow: false, reason: "active_loop"`
- No mission resolvable → `allow: true, reason: "no_mission"` (graceful fallback at `cli/status/mod.rs:38-49`)

Derived by `readiness::stop_allowed` (`readiness.rs:103-112`), projected by `cli/status/project.rs:72-99`. Ralph hook exit semantics (exit 2 iff allow=false, else exit 0) verified by `tests/ralph_hook.rs` (6/6 passing).

### Global flags

`--mission`, `--repo-root`, `--json`, `--dry-run`, `--expect-revision` are declared as globals at `cli/mod.rs:45-65` and appear in every `--help` output. Mission resolution precedence (`docs/cli-contract-schemas.md:76-82`) implemented in `core/mission::resolve_mission`.

## Reading map — where I verified each iter 3 mandate

| Mandate | Verified at |
| --- | --- |
| `plan.task_ids` field in `state::schema::PlanState` | `state/schema.rs:71-86` (shown above) |
| `plan check` writes `plan.task_ids` | `cli/plan/check.rs:95-114` (shown above); `check.rs:240,312` (Summary.task_ids population) |
| `readiness::tasks_complete` consults `plan.task_ids` | `state/readiness.rs:74-93` (shown above) |
| Callers of `tasks_complete` | one: `derive_verdict` at `state/readiness.rs:57`; see table above |
| No alternative `all tasks done` predicate that bypasses the fix | `close/check.rs:132-140` still uses the buggy pattern — F3 finding |
| iter 2 checks (surface, envelope, codes, agreement) | sections above |
