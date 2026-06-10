# CLI Contract Audit — iter 5

Branch audited: `main` @ `c5e07ad`
Audited on: 2026-04-20 UTC
Binary: `target/release/codex1 0.1.0` (built from `c5e07ad`).
Worktree: `.claude/worktrees/agent-a8f65e5b` (fresh checkout at `c5e07ad`, no source/test/skill/doc edits).

## Scope

Fresh pass on every command listed in `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface, plus the iter 5 verification list from the task prompt:

- Minimal command surface wired in clap (all 29 leaves).
- Success + error envelope shape stability.
- Error codes ⊆ canonical `CliError` set (grep + filter).
- `status` ↔ `close check` agreement across ≥5 hand-crafted STATE.json fixtures.
- One `tasks_complete`-style predicate (grep `tasks_complete|tasks_all_complete|fn.*all.*complete` across `src/**/*.rs`).
- F11 upgrade trap guard: `cli/plan/check.rs` contains `let task_ids_missing = …` AND the short-circuit predicate AND is negated (`!task_ids_missing`).
- `close check` blocker list emits `TASK_NOT_READY: <id> has not started` for un-started DAG nodes when `plan.task_ids` is populated.
- F11 regression test exists in `crates/codex1/tests/plan_check.rs`.
- The regression test passes when run in isolation (`cargo test --release --test plan_check plan_check_backfills`).

## Summary

**PASS — 0 P0, 0 P1, 0 P2.**

The iter 4 audit (`docs/audits/iter4-cli-contract-audit.md`) closed with one open P1 (F1: the F11 upgrade trap predicate was missing from `cli/plan/check.rs` despite iter-3 claiming to have added it). Commits `b212ca8` (iter4-fix) and `c5e07ad` (iter4-fix-followup) resolve that finding:

- `crates/codex1/src/cli/plan/check.rs:70-72` now reads

  ```rust
  let hash_matches = current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());
  let task_ids_missing = current.plan.task_ids.is_empty();
  let already_locked_same = hash_matches && !task_ids_missing;
  ```

  exactly the predicate iter 4 asked for.
- `crates/codex1/tests/plan_check.rs:661` defines `plan_check_backfills_missing_task_ids_and_then_stays_idempotent`, the regression guard.
- Empirical repro of the iter 4 scenario: first `plan check` locks at revision 3 with `task_ids=["T1","T2"]`; stripping `task_ids` to `[]` and re-running `plan check` now backfills (rev 3 → 4, one new `plan.checked` event); a third run is idempotent (rev stays 4, no event).

Every other iter 4 clean-check (CC-1 through CC-8) continues to hold at `c5e07ad`. The build gate is clean. All 170 tests pass (up from iter 4's 169 because of the new regression test).

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS (silent) |
| `cargo clippy --all-targets -- -D warnings` | PASS (0 errors, 0 warnings) |
| `cargo test --release` | PASS (170 passed / 0 failed / 0 ignored) |

### F11 regression test — isolated run

```text
$ cargo test --release --test plan_check plan_check_backfills
    Finished `release` profile [optimized] target(s) in 15.02s
     Running tests/plan_check.rs (target/release/deps/plan_check-…)

running 1 test
test plan_check_backfills_missing_task_ids_and_then_stays_idempotent ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.71s
```

## Findings

None.

## Clean checks

### CC-1: F1 (P1, iter 4) is resolved

- **Primary-source evidence — predicate landed.** `git show c5e07ad:crates/codex1/src/cli/plan/check.rs | grep task_ids_missing` (equivalently `git show origin/main:…`; note the worktree's local `main` branch is pinned elsewhere but `origin/main` and `c5e07ad` are the same commit) returns:

  ```text
  71:    let task_ids_missing = current.plan.task_ids.is_empty();
  72:    let already_locked_same = hash_matches && !task_ids_missing;
  ```

  `cli/plan/check.rs:61-72` (the short-circuit block) now reads:

  ```rust
  // Idempotent short-circuit: same hash on an already-locked plan → no mutation.
  //
  // Exception — upgrade-in-place: if a previously-locked state has no
  // `plan.task_ids` snapshot (pre-F8 binary, or hand-edited state),
  // fall through so the mutation closure backfills it. Without this
  // guard, an upgraded binary would perpetually re-enter the
  // short-circuit and `status` / `close check` would be stuck with
  // `verdict=continue_required` and no actionable blockers.
  let current = state::load(&paths)?;
  let hash_matches = current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());
  let task_ids_missing = current.plan.task_ids.is_empty();
  let already_locked_same = hash_matches && !task_ids_missing;
  ```

  This is exactly the fix iter 4 prescribed (`iter4-cli-contract-audit.md` § F1, "Fix sketch"). The short-circuit now additionally requires `!task_ids_missing`, so an upgraded binary seeing a pre-F8 lock falls through into the mutation closure at line 104 and backfills `plan.task_ids`.
- **Primary-source evidence — regression test landed.**

  ```text
  $ grep -l plan_check_backfills_missing_task_ids_and_then_stays_idempotent crates/codex1/tests/*.rs
  crates/codex1/tests/plan_check.rs
  ```

  The test (`tests/plan_check.rs:660-710`) locks a 4-task mission, strips `plan.task_ids` to `[]`, re-runs `plan check`, asserts the revision bumped and `task_ids` repopulated to `["T1","T2","T3","T4"]`, then re-runs again and asserts the revision did not bump (idempotent).
- **Empirical repro** (`target/release/codex1` built from `c5e07ad`, scratch mission at `/tmp/codex1-iter5-f11test`):

  1. Ran `init`, wrote valid OUTCOME + PLAN, ratified OUTCOME, chose level `light`, ran `plan check` → `ok=true`, `revision=3`, `plan.task_ids=["T1","T2"]`.
  2. Stripped `task_ids` to `[]` on disk (simulating a pre-F8 lock). STATE.json still shows `revision=3` and `plan.locked=true`.
  3. Re-ran `plan check` → `ok=true`, `revision=4`, `plan.task_ids=["T1","T2"]` repopulated. `EVENTS.jsonl` now contains a second `plan.checked` entry.
  4. Re-ran `plan check` → `ok=true`, `revision=4` (unchanged). No new event. Idempotent.

  This is the exact "first call mutates, second call is idempotent" cycle iter 4 asked for.

### CC-2: Minimal command surface wired in clap (29 leaves)

Every minimal-surface command resolves via `codex1 <group> <verb> --help`. Verified by invoking each leaf with `--help`:

| Group | Verbs | Source |
| --- | --- | --- |
| (root) | `init`, `status`, `doctor`, `hook snippet` | `cli/mod.rs:76-124`, `cli/init.rs`, `cli/status/mod.rs`, `cli/doctor.rs`, `cli/hook.rs` |
| `outcome` | `check`, `ratify` | `cli/outcome/mod.rs` |
| `plan` | `choose-level`, `scaffold`, `check`, `graph`, `waves` | `cli/plan/mod.rs` |
| `task` | `next`, `start`, `finish`, `status`, `packet` | `cli/task/mod.rs` |
| `review` | `start`, `packet`, `record`, `status` | `cli/review/mod.rs` |
| `replan` | `check`, `record` | `cli/replan/mod.rs` |
| `loop` | `pause`, `resume`, `deactivate`, `activate` | `cli/loop_/mod.rs` |
| `close` | `check`, `complete`, `record-review` | `cli/close/mod.rs` |

All 29 leaves return clap help without error. Total matches the `iter4-cli-contract-audit.md § CC-1` table (25 group-qualified plus 4 root verbs).

### CC-3: Success + error envelopes stable

- Success envelope (`core/envelope.rs`): `{ ok: true, mission_id?, revision?, data }`. Verified live on every command group during fixture runs below.
- Error envelope (`core/envelope.rs`): `{ ok: false, code, message, hint?, retryable, context? }`. `hint`, `message_context`-style `context`, and `retryable` fields serialize exactly once; `hint` and `context` use `skip_serializing_if` so the shape remains stable when absent.
- Sample error bodies seen live at `c5e07ad`: `OUTCOME_INCOMPLETE` (missing YAML frontmatter), `PLAN_INVALID` (bad task kind, missing section), `STATE_CORRUPT` (unknown Phase variant), `CLOSE_NOT_READY` (via `close check` payload not via error; see CC-5).

### CC-4: Error codes ⊆ canonical `CliError` set

- Canonical set (18 codes) at `core/error.rs:82-102`: `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`, `CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`, `STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`, `CONFIG_MISSING`, `MISSION_NOT_FOUND`, `PARSE_ERROR`, `NOT_IMPLEMENTED`.
- Grep across `crates/codex1/src/**/*.rs` for double-quoted uppercase tokens matched `"[A-Z_]{4,}"`; every hit that looks like an error code is one of:
  - A `CliError::code()` arm in `core/error.rs`.
  - A `Blocker::new("<CODE>", …)` call in `cli/close/check.rs` using `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `REPLAN_REQUIRED`, `TASK_NOT_READY`, `REVIEW_FINDINGS_BLOCK`, `CLOSE_NOT_READY`.
  - An `exit_with_validation_error("<CODE>", …)` call in `cli/plan/check.rs` using `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`.
  - A documentation / doc-comment reference (`core/envelope.rs:10`).
- No non-canonical code escapes. `INTERNAL`-style and `Other`-style variants are absent (previously dropped; no regression at `c5e07ad`).

### CC-5: `status` ↔ `close check` agreement across 10 hand-crafted fixtures

Each fixture below is a `STATE.json` written by hand to `/tmp/codex1-iter5-fx<name>/PLANS/demo/STATE.json`, with matching stub `OUTCOME.md` + `PLAN.yaml` so both commands load. `status --mission demo --json` and `close check --mission demo --json` were both invoked against the same `--repo-root`.

| # | Fixture | `status.verdict` | `close.verdict` | `status.close_ready` | `close.ready` |
| --- | --- | --- | --- | --- | --- |
| 1 | fresh `init` (outcome unratified, plan unlocked) | `needs_user` | `needs_user` | false | false |
| 2 | outcome ratified, plan unlocked | `needs_user` | `needs_user` | false | false |
| 3 | plan locked, T1 complete + T2 in_progress | `continue_required` | `continue_required` | false | false |
| 4 | all tasks complete, mission-close review not started | `ready_for_mission_close_review` | `ready_for_mission_close_review` | false | false |
| 5 | mission-close review passed | `mission_close_review_passed` | `mission_close_review_passed` | **true** | **true** |
| 6 | terminal (`close.terminal_at` set) | `terminal_complete` | `terminal_complete` | false | false |
| 7 | DAG with `T99` never started (plan locked) | `continue_required` | `continue_required` | false | false |
| 8 | dirty review on T2 blocks progress | `blocked` | `blocked` | false | false |
| 9 | replan.triggered=true | `blocked` | `blocked` | false | false |
| 10 | mission-close review open | `mission_close_review_open` | `mission_close_review_open` | false | false |

For every fixture: (a) `status.data.verdict == close.data.verdict`, (b) `status.data.close_ready == close.data.ready`. By construction both commands call `state::readiness::derive_verdict`, and `status` computes `close_ready` as `matches!(verdict, MissionCloseReviewPassed)` at `cli/status/mod.rs`, identical to `close check`'s `ready` at `cli/close/check.rs:49`.

Fixture 7 also exercises the `has not started` blocker branch (see CC-8). Fixtures 4, 5, 6, 9, 10 each produce a uniquely-named verdict string — they exercise distinct `derive_verdict` arms.

### CC-6: `tasks_complete` is the only `tasks_complete`-style predicate

Grep across `crates/codex1/src/**/*.rs`:

| File:line | Hit | Category |
| --- | --- | --- |
| `state/readiness.rs:80` | `pub fn tasks_complete(state: &MissionState) -> bool` | canonical definition |
| `state/readiness.rs:57` | `if tasks_complete(state) {` | internal use inside `derive_verdict` |
| `state/schema.rs:82` | `readiness::tasks_complete` (doc comment only) | doc reference |
| `cli/close/check.rs:132` | `if readiness::tasks_complete(state) {` | delegates to canonical predicate |
| `cli/plan/waves.rs:6, 73, 89, 106` | `all_tasks_complete` in the `plan waves --json` projection | derived JSON field recomputed each call; not a close-readiness predicate |

`cli/plan/waves.rs` uses `all_tasks_complete` as the JSON field name for a wave-list projection, and it is computed locally from `tasks[].depends_on` + `state.tasks` — not a mission-close predicate and never consulted by `status` / `close check`. Confirmed read-only: no `state::mutate`, `atomic_write`, or `fs::write` in `cli/plan/waves.rs`.

No parallel `tasks_all_complete` copy exists. The iter 3 wave-fix is intact at `c5e07ad`.

### CC-7: F11 upgrade trap guard is present and correct

- Primary-source evidence at `cli/plan/check.rs:71-72`:
  ```rust
  let task_ids_missing = current.plan.task_ids.is_empty();
  let already_locked_same = hash_matches && !task_ids_missing;
  ```
  Verified with `git show main:crates/codex1/src/cli/plan/check.rs | grep task_ids_missing` (two matches, at lines 71 and 72).
- The mutation closure at `cli/plan/check.rs:109-122` backfills `s.plan.task_ids.clone_from(&task_ids_to_record)` whenever the short-circuit is skipped.
- Empirical behavior at `c5e07ad`: see CC-1 for the full three-step repro. `plan.task_ids` backfills on the first re-run, and a second re-run is idempotent.

### CC-8: `close check` blocker list includes un-started DAG tasks

Empirical fixture (fx7): `plan.task_ids = ["T1","T2","T99"]`, `state.tasks = { T1: complete, T2: in_progress }`. `codex1 close check --json` returns:

```json
{
  "ok": true,
  "mission_id": "demo",
  "revision": 4,
  "data": {
    "blockers": [
      { "code": "TASK_NOT_READY", "detail": "T2 is in_progress" },
      { "code": "TASK_NOT_READY", "detail": "T99 has not started" }
    ],
    "ready": false,
    "verdict": "continue_required"
  }
}
```

Source: `cli/close/check.rs:109-123`. When `plan.task_ids` is non-empty, the block iterates each id and emits `TASK_NOT_READY: <id> has not started` for any id missing from `state.tasks`. The literal `"{id} has not started"` string lives at `cli/close/check.rs:119`.

The empty-`task_ids` fallback (lines 100-108) iterates `state.tasks` directly and emits per-task `TASK_NOT_READY`, preserving early-phase signal before `plan check` has locked.

## Reading map — iter 5 verification list versus this audit

| iter 5 scope line | Verified at |
| --- | --- |
| Minimal command surface wired | CC-2 |
| Envelopes (success + error) stable | CC-3 |
| Error codes ⊆ canonical `CliError` set | CC-4 |
| `status` ↔ `close check` agree across ≥5 hand-crafted STATE.json fixtures | CC-5 (10 fixtures) |
| One `tasks_complete`-style predicate | CC-6 |
| F11 upgrade trap guard actually in `cli/plan/check.rs` | CC-1, CC-7 |
| `close check` blocker list includes `TASK_NOT_READY: <id> has not started` | CC-8 |
| F11 regression test exists | CC-1 |
| The regression test passes | CC-1, build evidence |

Every iter 5 scope line passes. No source, test, skill, or existing doc was modified by this audit.
