# Round 2 — correctness-invariants audit

Baseline: `05fcae3`. Scope: `crates/codex1/src/`. Lens: runtime correctness
invariants (fs2 discipline, atomic-write, EVENTS ordering, strict
`--expect-revision`, dirty-counter rules, verdict ordering, panics,
`CliError` completeness, concurrency, round-1 TOCTOU follow-up, new
tests).

## Summary

Round 1 fixes verified correct on the load-bearing pieces:

- **EVENTS-before-STATE write order** (`state::mutate` at
  `src/state/mod.rs:126-145`) — events append, then STATE persist. The
  commentary at 126-141 documents the recoverable vs. silently-wrong
  tradeoff. Crash-recovery doctor remains read-only and does not
  truncate (confirmed via `Grep` in `src/cli/doctor.rs` — no write
  helpers invoked). Complete.
- **Parent-directory fsync after `persist`** in
  `state::fs_atomic::atomic_write` (`src/state/fs_atomic.rs:33-35`) —
  fires on every mission artifact write (STATE.json, PLAN.yaml,
  CLOSEOUT.md, OUTCOME.md, review findings). Complete.
- **`outcome ratify` OUTCOME.md atomicity**
  (`src/cli/outcome/ratify.rs:60-75`) — `state::mutate` runs first,
  then `atomic_write(&outcome_path, …)` lands outside the closure. A
  mid-closure panic no longer leaves OUTCOME.md flipped to `ratified`
  while STATE says otherwise. Complete.
- **`check_expected_revision` helper** — promoted to
  `state::check_expected_revision` (`src/state/mod.rs:40-53`) and
  wired into the nine short-circuit paths listed in the round-1
  decisions (confirmed via `Grep check_expected_revision`, 13 call
  sites). **Not complete**: see P2 below; `review start` was missed.
- **`require_plan_locked` guard**
  (`src/state/mod.rs:63-71`) wired into `task start`, `task finish`,
  `review start`, `review record` (confirmed via
  `Grep require_plan_locked`). The fix itself is correct — but the
  round-1 agent explicitly flagged the TOCTOU angle as a follow-up;
  see P1 below.
- **Verdict derivation** at `src/state/readiness.rs:40-66` matches
  `docs/cli-contract-schemas.md:179-190` line-for-line: terminal →
  ratified → locked → replan.triggered → any Dirty review → tasks
  complete {NotStarted/Open/Passed} → continue_required. No drift.
- **Dirty-counter rules** at `src/cli/review/record.rs:284-290` only
  dispatch `apply_clean`/`apply_dirty` on `AcceptedCurrent`. Clean
  resets (`apply_clean:307-311`), dirty bumps + checks the six-step
  threshold (`apply_dirty:338-353`). Reset on replan happens in
  `apply_replan` (`src/cli/replan/record.rs:150`). Invariants hold.
- **`CliError` completeness** — all 18 codes in
  `docs/cli-contract-schemas.md:46-63` are present as variants in
  `src/core/error.rs:24-76`. The five reserved/edge variants
  (`ConfigMissing`, `NotImplemented`, `ReplanRequired`, and the
  transparent `Io`/`Json`/`Yaml` passthroughs) are now unit-tested at
  `src/core/error.rs:181-254`.
- **Concurrent-writer regression**
  (`tests/foundation.rs:237-313`) genuinely races two processes on
  `loop activate --expect-revision 0`, asserts one succeeds + one
  `REVISION_CONFLICT`, asserts final revision is 1 and EVENTS.jsonl
  has exactly one line with `seq=1`. Correct — though note it pins
  both workers to `--expect-revision 0` and therefore does NOT
  exercise the race in P1 below.
- **Panics/unwraps/expects** — the only non-test runtime `expect` is
  at `src/cli/plan/dag.rs:51` (`indegree.get_mut(child).expect(...)`);
  invariant holds by construction (`child` comes from `succ[id]` and
  every key in `succ` was inserted into `indegree` at lines 27-30).
  The other non-test `expect` is inside a JSON-literal builder at
  `src/cli/plan/choose_level.rs:162`, documented with a comment that
  names the constructor guarantee. No `panic!`/`unreachable!`/`todo!`
  in runtime paths.

New round-1 test artifacts checked:

- `src/state/readiness.rs::tests` — 11 unit tests as claimed. Each
  constructs a `MissionState::fresh` with exactly the relevant fields
  flipped. `derive_verdict_terminal_complete_wins_over_everything`
  proves the `close.terminal_at` precondition beats every other
  clause. `tasks_complete_requires_dag_and_records` walks the empty /
  missing-record / complete / superseded / in-progress branches.
  Coverage faithful to the assertion.
- `src/core/error.rs::tests` — 5 unit tests that construct each
  reserved-variant envelope and assert `code`, `retryable`, `hint`,
  `context["..."]` shape. Correct.
- `tests/foundation.rs` concurrent/envelope/YAML tests — all three
  exercise the invariants they claim (see notes above).
- `tests/plan_scaffold.rs` escalation tests — assert
  `escalation_required: false` + absent `escalation_reason` when
  requested equals effective, and the opposite on bump. Correct.
- `tests/plan_check.rs::review_loop_deadlock_returns_plan_invalid` —
  constructs the precise deadlock shape at round-1 e2e P2-1,
  asserts `PLAN_INVALID`, and confirms the plan stays unlocked after
  a second attempt (no lock leakage). Correct.
- `tests/replan.rs::task_start_after_replan_record_refuses_with_plan_invalid`
  — seeds an unlocked post-replan state and confirms `task start`
  returns `PLAN_INVALID` with a "plan check" hint, then that flipping
  `plan.locked` back lifts the block. Correct.
- `tests/status.rs::unlocked_plan_emits_empty_ready_tasks_and_review_required`
  + `blocked_surfaces_awaiting_review_when_plan_is_valid` — both
  assert the intended shape; no drift.
- `tests/review.rs::late_same_boundary_does_not_bump_or_reset_dirty_counter`
  — seeds counter at 3, drives a late-boundary dirty record,
  asserts counter still 3 and `replan.triggered` still false.
  Correct.
- `tests/close.rs::record_review_open_then_clean_transitions_to_passed`
  — first records dirty (→ Open), then clean (→ Passed); asserts
  both transitions and the `__mission_close__` counter stays at 1
  after clean. Correct.

Two findings remain for round 2.

## P0

None.

## P1

### P1-1 · `require_plan_locked` TOCTOU between load and mutate allows state corruption

**Evidence.**

- `src/state/mod.rs:63-71` `require_plan_locked` operates on a caller-supplied snapshot.
- `src/cli/task/start.rs:18-22` loads state with the shared lock, calls `require_plan_locked(&state)`, drops the lock, then calls `state::mutate` (line 99). The mutate closure at `start.rs:104-110` runs `ensure_task_record(state, &task_id); rec.status = TaskStatus::InProgress;` and does NOT re-check `plan.locked`.
- `src/cli/task/finish.rs:18-21` + closure at `finish.rs:108-114` — same pattern.
- `src/cli/review/start.rs:33-41` + closure at `review/start.rs:86-106` — same pattern.
- `src/cli/review/record.rs:71-78` + closure at `review/record.rs:179-189`; the closure re-classifies against the fresh state (237-240) but never re-checks `plan.locked`.
- `docs/cli-contract-schemas.md:74` lists `--expect-revision` as "Mutating commands only" — it is not required on every mutation. The safe path a caller would need to opt into is not a contract invariant.

**Reproduction (process-level, no `--expect-revision`).**

1. STATE at revision N with `plan.locked = true`, `tasks["T1"].status = Ready`.
2. Thread A: `codex1 task start T1` loads the state at rev N under shared lock, passes `require_plan_locked`, releases the lock, races for the exclusive lock.
3. Thread B wins the exclusive lock first: `codex1 replan record --reason six_dirty --supersedes T1`. Sets `tasks["T1"].status = Superseded`, `tasks["T1"].superseded_by = "replan-N"`, `plan.locked = false`, bumps to rev N+1.
4. Thread A acquires the exclusive lock, re-reads rev N+1. No `--expect-revision` to trip. Closure runs `ensure_task_record(state, "T1")`, finds the existing Superseded record, flips `status = InProgress` and `started_at = Some(now)` (`src/cli/task/start.rs:105-108`). `superseded_by` is untouched.
5. Post-mutation state has `tasks["T1"].status = InProgress && superseded_by = "replan-N"`, `plan.locked = false`, plus the `task.started` event on `EVENTS.jsonl` — a task started against a plan that is no longer locked, sitting on top of a replan-superseded record.

The `tests/foundation.rs::concurrent_loop_activate_serializes_via_fs2_lock` test does not cover this race — both its workers pass `--expect-revision 0`, so the in-mutate revision check catches the loser. The race surfaces only when the caller omits `--expect-revision`, which the contract permits.

**Severity.** P1. Real state corruption; no data loss, but the resulting record is a shape `status=InProgress, superseded_by=<marker>` that no other code path constructs or cleans up, and the "started task on unlocked plan" is the exact invariant the round-1 fix was meant to close.

**Remediation sketch (for the fixer, not part of this audit).** Move the guard into the mutate closure. The closure already has a `&mut MissionState` under the exclusive lock; re-run `require_plan_locked(state)` at its top. Same for `close.terminal_at` on `review start`. The pre-load check can stay as a fast-fail optimization.

## P2

### P2-1 · `review start` dry-run path skips `check_expected_revision`

**Evidence.**

- Round-1 decisions `docs/audits/round-1/decisions.md:13` enumerate every short-circuit that was wired up: "`task/start.rs` (idempotent + dry-run), `task/finish.rs`, `plan/check.rs`, `plan/choose_level.rs`, `review/record.rs`, `close/complete.rs`, `close/record_review.rs` (clean + dirty), `outcome/ratify.rs`, `loop_/mod.rs`." `review/start.rs` is not in the list.
- `src/cli/review/start.rs:61-74` is the dry-run branch and it emits success without calling `state::check_expected_revision`:
  ```
  if ctx.dry_run {
      let env = JsonOk::new(Some(state.mission_id.clone()), Some(state.revision), json!({...}));
      println!("{}", env.to_pretty());
      return Ok(());
  }
  ```
- `Grep check_expected_revision` in `src/cli/review/start.rs` → zero hits.

**Impact.** A caller passing `--expect-revision N --dry-run` on `review start` gets an `ok:true` envelope even when the on-disk revision has moved past N. The strict-equality invariant at `docs/cli-contract-schemas.md:74` applies to "Mutating commands only" but the round-1 decision treated dry-run of mutating commands as falling under the rule (see `task/start.rs:78` and `close/complete.rs:53`). The inconsistency is the bug — one path honors it, another doesn't.

No safety regression (dry-run makes no on-disk change), but the round-1 claim that the helper was wired into "every short-circuit" is not actually true.

**Severity.** P2. Missing enforcement of a documented invariant on one command; no data-loss path. Dedupe target for any future round that re-opens "short-circuit completeness."

## P3

### P3-1 · `src/cli/plan/dag.rs:51` `.expect("indegree entry")` lacks an invariant comment

Round-1 REJECT bucket already includes "non-blocking P3 · `.expect` calls without invariant comments" (decisions.md:38). Re-noting for completeness in case a later loop tightens the rule: the `.expect` at line 51 is safe by the construction at lines 27-30 and 36 (every key in `succ` was also inserted into `indegree` during initialization), but a one-line "// child is in succ[id] implies child is in indegree — populated together at …" would satisfy the round-1 lint without producing a real runtime check. Not in scope for round 2.

### P3-2 · `replan record` dry-run + `plan scaffold` dry-run re-implement `check_expected_revision` inline

Both `src/cli/replan/record.rs:41-48` and `src/cli/plan/scaffold.rs:27-34` open-code the `expected != state.revision` check instead of calling `state::check_expected_revision`. The invariant holds — strict equality — but calling the helper would be stylistically consistent with the other dry-run branches and reduce future drift risk. Non-blocking; same category as P3-1.

### P3-3 · `close/record_review.rs` dirty path: state mutate precedes findings-file write

`src/cli/close/record_review.rs:183-211` mutates STATE (bumping the counter and potentially flipping `replan.triggered`) before writing `reviews/mission-close-<rev>.md`. The comment at 177-181 notes the design choice: "state is authoritative; if the findings file write fails the bumped counter remains." The shape mirrors the round-1 OUTCOME ratify pattern (state mutate inside `state::mutate`, auxiliary file written outside the closure) and is defensible — state is truth, findings file is secondary. Worth a one-line "see also: outcome/ratify.rs ordering commentary" cross-reference so future readers don't re-derive the choice. Non-blocking.
