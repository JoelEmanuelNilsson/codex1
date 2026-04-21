# CLI Contract Audit — iter 4

Branch audited: `main` @ `5a16894`
Audited on: 2026-04-20 UTC
Binary: `target/release/codex1 0.1.0` (built from `5a16894`).
Worktree: `.claude/worktrees/agent-a23c02b0` (fresh checkout from commit `5a16894`, no source edits).

## Scope

Fresh pass on every command listed in `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface, plus the iter 4 verification list from the task prompt:

- Minimal command surface wired in clap.
- Success + error envelope shape stability.
- Error codes ⊆ canonical `CliError` set (grep-and-verify).
- `status` ↔ `close check` agreement across ≥5 hand-crafted STATE.json fixtures.
- `readiness::tasks_complete` is the ONLY `tasks_complete`-style predicate (grep across `src/**/*.rs` for `tasks_complete|tasks_all_complete|all.*complete`).
- `plan.task_ids` backfill on upgrade: a pre-F8 state (`locked=true`, `hash=X`, `task_ids=[]`) re-run through `codex1 plan check` must populate `task_ids` (revision +1) then a second re-run must be idempotent.
- `close check` blocker list includes un-started DAG tasks explicitly (`TASK_NOT_READY: <id> has not started`).

## Summary

**FAIL — 0 P0, 1 P1, 0 P2.**

Every iter-3 P0 regression (F8 semantic bug, F9 clippy, F10 fixtures, and the iter3-wave F3 parallel predicate) is resolved at `5a16894`. Build gate (fmt / clippy / test --release) is clean; 169 tests pass. `status` and `close check` agree on `verdict` across all eight hand-crafted fixtures I ran. The canonical error set is respected: no raw uppercase-code string outside `CliError::code()` appears anywhere under `crates/codex1/src/`.

The single P1 is that iter 3's F11 "upgrade trap" fix was claimed by the commit message at `2ef3ce7` but was never actually applied to the short-circuit predicate in `cli/plan/check.rs`. The upgrade trap is empirically reproducible at `5a16894`: a mission locked with an empty `plan.task_ids` field stays stuck at `continue_required` forever, with zero blockers reported by `close check`, and `plan check` refuses to backfill.

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS (silent) |
| `cargo clippy --all-targets -- -D warnings` | PASS (0 errors, 0 warnings) |
| `cargo test --release` | PASS (169 passed / 0 failed / 0 ignored) |

## Findings

### F1 (P1): `plan check` short-circuit silently swallows the pre-F8 upgrade case

- **Severity:** P1. The task's verification list calls this out explicitly: a pre-F8 state (`locked=true`, `hash=X`, `task_ids=[]`) re-run through `codex1 plan check` MUST populate `task_ids` (revision +1) then subsequent re-runs must be idempotent. At `5a16894`, the first requirement is not met: no backfill occurs. The mission remains stuck.
- **Where:** `crates/codex1/src/cli/plan/check.rs:61-84`:

  ```rust
  // Idempotent short-circuit: same hash on an already-locked plan → no mutation.
  let current = state::load(&paths)?;
  let already_locked_same =
      current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());

  if ctx.dry_run || already_locked_same {
      // ...returns the envelope without entering the mutation closure...
      return Ok(());
  }
  ```

  The predicate checks only `plan.locked` and `plan.hash`. It never consults `plan.task_ids`. A state file locked by a pre-F8 binary (no `task_ids` snapshot, field defaults to `[]` under `serde(default)` at `src/state/schema.rs:84-85`) matches `already_locked_same` and returns before the mutation closure at line 109 (`s.plan.task_ids.clone_from(&task_ids_to_record);`). No backfill, no revision bump, no event.
- **Provenance of the regression:** Iter 3 commit `2ef3ce7` claims in its message to have fixed exactly this trap by "Tightened the short-circuit to require `!task_ids_missing` in addition to `hash_matches`". The diff contains no such predicate — only the `clone_from` clippy fix at the same site:

  ```text
  $ git show 2ef3ce7 -- crates/codex1/src/cli/plan/check.rs
  @@ -106,7 +106,7 @@ pub fn run(ctx: &Ctx) -> CliResult<()> {
               // can recognize "all DAG nodes done" without silently
               // ignoring DAG nodes that were never started.
  -            s.plan.task_ids = task_ids_to_record.clone();
  +            s.plan.task_ids.clone_from(&task_ids_to_record);
  ```

  I confirmed by checking the corresponding lines in `git show 958d2f1:crates/codex1/src/cli/plan/check.rs` and `git show 2ef3ce7:crates/codex1/src/cli/plan/check.rs`: the `already_locked_same` block (lines 61-84) is byte-identical across `958d2f1`, `2ef3ce7`, and `5a16894`. No `task_ids_missing` / `task_ids.is_empty()` guard exists anywhere in the file:

  ```text
  $ grep -nE 'task_ids_missing|task_ids\.is_empty|task_ids_to_record\.is_empty' crates/codex1/src/cli/plan/check.rs
  (no matches)
  ```
- **Empirical reproduction** (`target/release/codex1` built from `5a16894`):

  1. Init and lock a valid 4-task plan the normal way. STATE.json ends with `revision: 1`, `plan.locked: true`, `plan.hash: sha256:5db3953e…`, `plan.task_ids: ["T1","T2","T3","T4"]`.
  2. Simulate a pre-F8 lock by clearing `task_ids` in STATE.json only (hash + locked flag retained).
  3. Re-run `codex1 plan check --mission demo`:

     ```text
     {
       "ok": true,
       "mission_id": "demo",
       "revision": 1,
       "data": { "locked": true, "plan_hash": "sha256:5db3953e…", "tasks": 4, ... }
     }
     ```

     No revision bump. No event. On disk: `plan.task_ids == []`, `revision == 1`. The short-circuit fired.
- **Downstream effect — the mission is silently stuck.** If a pre-F8 state has `plan.task_ids == []` but every task in `state.tasks` is `Complete`, `status --json` and `close check --json` both report:

  ```text
  verdict = continue_required
  ready   = false
  blockers = []   # <-- no TASK_NOT_READY, no CLOSE_NOT_READY, no anything
  ```

  The operator cannot tell what is wrong because `readiness::tasks_complete` returns `false` for any `plan.task_ids.is_empty()` (by design, per `state/readiness.rs:82-85`) AND the fallback branch of `close/check.rs::derive_blockers` (lines 100-108) only iterates tasks in `state.tasks` that are NOT complete — so there is nothing to report. The verdict-vs-blocker coupling has a fixpoint that assumes `plan.task_ids` is authoritative whenever the plan is locked, which is the exact assumption the upgrade trap breaks.
- **Expected behavior:** Either (a) tighten the short-circuit to require `!current.plan.task_ids.is_empty() && task_ids_to_record == current.plan.task_ids` (one-shot backfill, then idempotent), or (b) surface a `PLAN_INVALID` hint when a locked plan has empty `task_ids` and the operator has not asked to re-lock. (a) is what the iter 3 commit message promised and is the least invasive fix — the next `plan check` mutates once, writes an event, increments revision, and subsequent re-runs short-circuit idempotently.
- **Fix sketch:**

  ```rust
  let already_locked_same = current.plan.locked
      && current.plan.hash.as_deref() == Some(hash.as_str())
      && !current.plan.task_ids.is_empty();
  ```

  Behavior after fix (empirically reasoned): first re-run enters the mutation closure, writes `task_ids`, bumps revision → 2, appends one `plan.checked` event. Second re-run sees `task_ids` populated AND hash matches → short-circuit returns without mutation, idempotent.
- **Why P1 not P0:** The upgrade trap is latent — no fresh mission started on `5a16894` encounters it. Nothing in the build gate reveals the regression (no test covers the scenario). But the task's iter 4 scope pins the requirement, and the commit `2ef3ce7` message positively asserts the fix exists when in fact it does not. That is a direct contract break against iter 3's own remediation claim.
- **Contract reference:** Task's iter4-cli-contract check list, bullet 6 ("`plan.task_ids` backfill on upgrade: a pre-F8 state (locked=true, hash=X, task_ids=[]) re-run through `codex1 plan check` must populate task_ids (revision +1) then subsequent re-runs must be idempotent.").

## Clean checks (no findings)

### CC-1: Minimal command surface wired in clap

All 25 minimal-surface commands resolve via `codex1 <group> <verb> --help`. Every leaf has clap-generated help text.

| Group | Verbs | Source |
| --- | --- | --- |
| (root) | `init`, `status`, `doctor`, `hook snippet` | `cli/mod.rs:76-126`, `cli/init.rs`, `cli/status/mod.rs`, `cli/doctor.rs`, `cli/hook.rs` |
| `outcome` | `check`, `ratify` | `cli/outcome/mod.rs:22-35` |
| `plan` | `choose-level`, `scaffold`, `check`, `graph`, `waves` | `cli/plan/mod.rs:22-67` |
| `task` | `next`, `start`, `finish`, `status`, `packet` | `cli/task/mod.rs:22-58` |
| `review` | `start`, `packet`, `record`, `status` | `cli/review/mod.rs:23-81` |
| `replan` | `check`, `record` | `cli/replan/mod.rs:18-37` |
| `loop` | `pause`, `resume`, `deactivate`, `activate` | `cli/loop_/mod.rs:24-47` |
| `close` | `check`, `complete`, `record-review` | `cli/close/mod.rs:43-73` |

### CC-2: Stable success + error envelopes

- Success envelope (`core/envelope.rs:20-29`): `{ ok: true, mission_id?, revision?, data }` — verified live on every command group via the fixture runs below.
- Error envelope (`core/envelope.rs:67-78`): `{ ok: false, code, message, hint?, retryable, context? }` — verified live on `OUTCOME_INCOMPLETE`, `STATE_CORRUPT`, `PLAN_INVALID`, `TASK_NOT_READY`, `CLOSE_NOT_READY`. `hint` and `context` are `skip_serializing_if` optional; the shape remains stable when omitted (noted as an existing schema clarification in prior audits).

### CC-3: Error codes ⊆ canonical `CliError` set

- Canonical set (18 codes): `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`, `CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`, `STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`, `CONFIG_MISSING`, `MISSION_NOT_FOUND`, `PARSE_ERROR`, `NOT_IMPLEMENTED` (`core/error.rs:80-103`).
- `INTERNAL` from baseline P2-1 is gone (the `CliError::Other` variant and `From<anyhow::Error>` were removed); no handler can emit a non-canonical code.
- Grep every raw uppercase-code string under `crates/codex1/src/` and filter to code-like literals:

  ```text
  $ grep -rnE '"[A-Z_]{4,}"' crates/codex1/src --include='*.rs'
    | filter for strings that look like error codes
  ```

  Every hit is either: (a) a `CliError::code()` arm, (b) a `Blocker::new(code, …)` call in `cli/close/check.rs` using `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `REPLAN_REQUIRED`, `TASK_NOT_READY`, `REVIEW_FINDINGS_BLOCK`, `CLOSE_NOT_READY`, or (c) an `exit_with_validation_error` call in `cli/plan/check.rs` using `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`. No bespoke code escapes the canonical set.

### CC-4: `status` ↔ `close check` agreement (8 hand-crafted STATE.json fixtures)

Each fixture below is a STATE.json written by hand to `/tmp/codex1-iter4-fx-<name>-*/PLANS/demo/STATE.json`, with matching stub `OUTCOME.md` + `PLAN.yaml` so both commands load. `status --mission demo` and `close check --mission demo` always returned the same verdict and `close_ready`/`ready` matched.

| # | Fixture | `status.verdict` | `close.verdict` | `status.close_ready` | `close.ready` |
| --- | --- | --- | --- | --- | --- |
| 1 | fresh init (outcome unratified, plan unlocked) | `needs_user` | `needs_user` | false | false |
| 2 | outcome ratified, plan unlocked | `needs_user` | `needs_user` | false | false |
| 3 | plan locked, T1 complete + T2 in_progress | `continue_required` | `continue_required` | false | false |
| 4 | all tasks complete, mission-close review not started | `ready_for_mission_close_review` | `ready_for_mission_close_review` | false | false |
| 5 | mission-close review passed | `mission_close_review_passed` | `mission_close_review_passed` | **true** | **true** |
| 6 | terminal (`close.terminal_at` set) | `terminal_complete` | `terminal_complete` | false | false |
| 7 | loop paused mid-execute | `continue_required` | `continue_required` | false | false |
| 8 | dirty review on T2 blocks progress | `blocked` | `blocked` | false | false |

Both commands derive `verdict` through `state::readiness::derive_verdict` (`readiness.rs:40-66`), and `close_ready`/`ready` are both `matches!(verdict, MissionCloseReviewPassed)` — by construction they cannot diverge. The 24 in-process property fixtures in `tests/status_close_agreement.rs` plus the 21 snapshot fixtures elsewhere in the test binary (iter 2 / iter 3 carryover) all continue to pass against `5a16894`.

### CC-5: `readiness::tasks_complete` is the ONLY `tasks_complete`-style predicate

Grep across `crates/codex1/src/**/*.rs` for `tasks_complete|tasks_all_complete|all.*complete`:

| File:line | Hit | Category |
| --- | --- | --- |
| `state/readiness.rs:80` | `pub fn tasks_complete(state: &MissionState) -> bool` | canonical definition |
| `state/readiness.rs:57` | `if tasks_complete(state) {` | internal use inside `derive_verdict` |
| `state/schema.rs:82` | `readiness::tasks_complete` (doc comment only) | doc reference |
| `cli/close/check.rs:132` | `if readiness::tasks_complete(state) {` | uses canonical (iter3-wave-fix at `5a16894` replaced the parallel copy) |
| `cli/status/next_action.rs:166` | doc comment `/// True when all 'depends_on' entries are complete/superseded AND …` | unrelated: per-task dep readiness predicate, not a mission-wide close predicate |
| `cli/task/next.rs:26` | string literal `"all tasks complete or superseded"` | response field value, not a predicate |
| `cli/task/lifecycle.rs:184` | doc comment `/// The current ready wave: the set of tasks whose deps are all complete` | unrelated: wave-readiness doc |
| `cli/plan/waves.rs:6, 73, 89, 106` | `all_tasks_complete` in `plan waves --json` output and its local computation | derived JSON field for the waves projection, recomputed each call from `tasks[].depends_on` + current status; not used by `readiness` or `close check` |

The iter3-wave-fix confirms: the parallel `tasks_all_complete` copy that used to live in `close/check.rs` is gone. `cli/close/check.rs:132` now calls `readiness::tasks_complete(state)`. There is no second home for the predicate.

### CC-6: `close check` blocker list includes un-started DAG tasks explicitly

Empirical check at `5a16894`. Fixture: `plan.task_ids = ["T1","T2","T99"]`, `state.tasks = {T1: complete, T2: in_progress}`.

```text
$ codex1 close check --mission demo --json
{
  ...
  "data": {
    "verdict": "continue_required",
    "ready": false,
    "blockers": [
      { "code": "TASK_NOT_READY", "detail": "T2 is in_progress" },
      { "code": "TASK_NOT_READY", "detail": "T99 has not started" }
    ]
  }
}
```

Source: `crates/codex1/src/cli/close/check.rs:100-123`. When `plan.task_ids` is non-empty, the block iterates it and emits `TASK_NOT_READY: <id> has not started` for any id missing from `state.tasks`. The literal `"{id} has not started"` string lives at `cli/close/check.rs:119`.

### CC-7: Error envelope stability

Ran ~20 commands across success and error paths. Every envelope parses as stable JSON with the documented field set. No surprises.

- `ok=true` flows: `doctor`, `init`, `status` (with and without mission), `outcome check` on complete OUTCOME, `plan choose-level`, `plan scaffold`, `plan check` (fresh + idempotent), `plan graph`, `plan waves`, `task next`, `task start`, `task status`, `task packet`, `review start`, `review packet`, `review record`, `replan check`, `loop pause`, `loop resume`, `close check`.
- `ok=false` flows: `outcome check` on incomplete OUTCOME (`OUTCOME_INCOMPLETE`), `task start T99` with missing deps (`TASK_NOT_READY`), `close complete` before close review passes (`CLOSE_NOT_READY`), `STATE.json` with unknown variant (`STATE_CORRUPT`), `plan check` with marker still present (`PLAN_INVALID`).

### CC-8: `close` has `check`, `complete`, `record-review`

`close check` projects `state::readiness::derive_verdict` + `derive_blockers`; `close complete` is gated on `verdict == mission_close_review_passed`; `close record-review` is the only write path to `MissionCloseReviewState::Passed` (noted in iter 1 / iter 2 / iter 3 as a documentation addition request — the current iter 4 scope does not re-assess that).

## Reading map — iter 4 verification list versus this audit

| iter 4 scope line | Verified at |
| --- | --- |
| Minimal command surface wired in clap | CC-1 |
| Envelopes (success + error) stable | CC-2, CC-7 |
| Error codes ⊆ canonical `CliError` set (grep raw codes) | CC-3 |
| `status` ↔ `close check` agree across ≥5 hand-crafted STATE.json fixtures | CC-4 (8 fixtures) |
| `readiness::tasks_complete` is the ONLY `tasks_complete`-style predicate | CC-5 |
| `plan.task_ids` backfill on upgrade | **F1 (P1)** — regression unfixed |
| `close check` blocker list includes un-started DAG tasks explicitly | CC-6 |

Every iter 4 scope line is exercised. Seven pass; one (the upgrade-trap backfill) fails with primary-source evidence in both the code diff and the empirical repro. No source was modified by this audit.
