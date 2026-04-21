# Round 3 — correctness-invariants audit

Baseline: `95451e6` (`v3 coming` on `main`). Scope: `crates/codex1/src/`.
Lens: runtime correctness invariants — fs2 lock discipline, atomic write,
EVENTS ordering, strict `--expect-revision`, dirty counter rules,
verdict derivation ordering, panics/unwraps/expects, `CliError`
completeness, concurrency, and the round-2 regression points listed in
the task.

## Summary

Round 2 fixes verified correct on every load-bearing surface.

Round-2 regression check results:

- **`cli/plan/check.rs` replan clear on relock**
  (`src/cli/plan/check.rs:133-134`) — mutation closure sets
  `s.replan.triggered = false; s.replan.triggered_reason = None;`
  unconditionally alongside the lock/hash/phase mutations. Both the
  narrow regression test (`tests/e2e_replan_trigger.rs::plan_check_after_replan_record_clears_triggered`)
  and the full reproducer (`full_mission_close_after_replan_reaches_terminal`)
  exercise this. Complete.
- **`require_plan_locked` re-checks inside `state::mutate` closures**:
  - `task/start.rs:111` — under the exclusive lock, before any state
    mutation. Comment at 105-110 documents the TOCTOU rationale.
  - `task/finish.rs:112` — same pattern, comment at 109-111.
  - `review/start.rs:95` — same pattern, comment at 92-94.
  - `review/record.rs:185-187` — guarded by `close.terminal_at.is_none()`
    so the terminal-contamination classification still wins precedence.
    Comment at 180-184 documents the round-2 decision. Every call
    happens inside the closure (after `state::mutate` acquires the
    exclusive lock), not before it.
- **`cli/task/next.rs` short-circuit ordering**
  (`src/cli/task/next.rs:26-53`) — `!state.plan.locked` is checked
  first (emits `{kind:"plan"}`), then `state.replan.triggered` is
  checked (emits `{kind:"replan"}`). Matches the round-2 e2e P2-1 spec
  and the round-1 `cli/status/project.rs::derive_next_action` ordering.
  Tests `task_next_unlocked_plan_emits_plan_kind` and
  `task_next_replan_triggered_emits_replan_kind` at
  `tests/status.rs:514,547` exercise both branches.
- **`cli/review/start.rs` dry-run `check_expected_revision`**
  (`src/cli/review/start.rs:66`) — called as first action inside the
  `if ctx.dry_run` branch. Comment at 62-65 explains the round-2
  correctness P2-1 fix. Test `review_start_dry_run_enforces_expect_revision`
  at `tests/review.rs:543` asserts `REVISION_CONFLICT` on mismatch.
- **New regression tests (6 added in round 2)**:
  - `tests/e2e_replan_trigger.rs::plan_check_after_replan_record_clears_triggered`
    — seeds a post-replan state, runs `plan check`, asserts
    `replan.triggered == false` and `triggered_reason == null`
    post-relock. Meaningful — would fail without the round-2 e2e P0-1
    fix.
  - `tests/e2e_replan_trigger.rs::full_mission_close_after_replan_reaches_terminal`
    — drives the mission through replan → relock → new-work → review
    → mission-close → `close complete`. Full round-trip proves the P0
    fix is not just narrow. Meaningful.
  - `tests/foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant`
    (lines 356-491) — spawns two processes racing `task start T2` vs.
    `replan record --supersedes T2 --reason six_dirty` across four
    iterations, asserts the final on-disk state never has
    `!plan.locked && tasks.T2.status == "in_progress"`. Crucially,
    runs WITHOUT `--expect-revision` so the invariant must come from
    the closure re-check, not from the revision guard. Meaningful.
  - `tests/status.rs::task_next_unlocked_plan_emits_plan_kind` —
    seeds `plan.locked=false` with ready task records that would
    otherwise surface as a wave; asserts `kind: plan` wins.
    Meaningful.
  - `tests/status.rs::task_next_replan_triggered_emits_replan_kind`
    — seeds `plan.locked=true` + `replan.triggered=true`; asserts
    `kind: replan` with the triggered reason. Meaningful.
  - `tests/review.rs::review_start_dry_run_enforces_expect_revision`
    — passes `--dry-run --expect-revision 999`, asserts
    `REVISION_CONFLICT`. Meaningful.

Other invariants spot-checked on this baseline:

- **fs2 lock discipline** (`state::mutate`,
  `src/state/mod.rs:91-152`) — exclusive lock acquired at 101, STATE
  re-read inside the lock at 109, `--expect-revision` check at
  114-122, mutator runs, event appended, STATE persisted, lock
  released at 146. No mutation path writes STATE.json without going
  through `state::mutate` (verified via `Grep paths.state()`;
  `state/mod.rs:145` and `state/mod.rs:175` are the only writers —
  the latter is `init_write`, pre-STATE).
- **Atomic write** (`src/state/fs_atomic.rs:22-37`) — tempfile in the
  same dir + `sync_data` on tempfile + `persist` (rename) + parent-dir
  `sync_all`. The task spec says `fsync tempfile`; the impl uses
  `sync_data` which flushes user data but not all inode metadata —
  sufficient for STATE.json/PLAN.yaml/OUTCOME.md/CLOSEOUT.md durability
  since rename() is the durability boundary, not tempfile metadata.
  No P-grade finding.
- **EVENTS.jsonl append-only, monotonic, appended before STATE persist**
  (`state::mutate`, `src/state/mod.rs:126-145`) — `state.events_cursor`
  is bumped before the event is constructed, then appended, then STATE
  is persisted. Comment at 126-141 documents the recoverable-vs-silent
  tradeoff. `append_event` opens the file with
  `.create(true).append(true)` and fsyncs via `sync_data` per line.
- **`--expect-revision` strict equality** via `state::check_expected_revision`
  (`src/state/mod.rs:40-53`) — wired into every idempotent/dry-run
  short-circuit confirmed in round 1, plus `review start` dry-run in
  round 2. All four loop transitions (`activate`/`pause`/`resume`/
  `deactivate`) funnel through `loop_::run_transition` which calls
  `check_expected_revision` on the Reject/NoOp/Apply(dry-run) paths
  and relies on `state::mutate` for the Apply(wet) path
  (`src/cli/loop_/mod.rs:62-119`). Complete.
- **Dirty counter rules**
  (`src/cli/review/record.rs:289-298`) — only `AcceptedCurrent`
  dispatches to `apply_clean`/`apply_dirty`. `LateSameBoundary`,
  `StaleSuperseded`, and `ContaminatedAfterTerminal` fall through with
  no counter effect. Reset on replan at
  `src/cli/replan/record.rs:150`. Invariants hold.
- **Verdict derivation ordering**
  (`src/state/readiness.rs:40-66`) — exactly matches
  `docs/cli-contract-schemas.md:179-190`: terminal → !ratified →
  !locked → replan.triggered → any Dirty review → tasks_complete
  {NotStarted/Open/Passed} → continue_required. No drift.
- **Panics/unwraps/expects** — the only non-test `unwrap` / `expect`
  in runtime paths are `src/cli/plan/dag.rs:51`
  (`.expect("indegree entry")`, safe by construction — `child` keys
  come from `succ[id]` which only holds ids also inserted into
  `indegree` at lines 27-30) and `src/cli/plan/choose_level.rs:162`
  (`.expect("build_payload constructs a JSON object literal")`, the
  `json!({…})` literal always returns `Value::Object`). Everything
  else is in `#[cfg(test)]` blocks. No new round-1/round-2 commit
  introduced a runtime `unwrap`/`expect` without an invariant comment
  (the dag.rs one predates the audit loop — already rejected as P3 in
  rounds 1 and 2).
- **`CliError` completeness** — 18 variants in
  `src/core/error.rs:24-76`, every one mapped to a stable `code()`
  string at 81-103 and to an envelope at 168-176. Reserved variants
  (`ConfigMissing`, `NotImplemented`, `ReplanRequired`,
  `RevisionConflict` shape stability, `Io→PARSE_ERROR+BUG` mapping)
  are unit-tested in `src/core/error.rs:191-253`. Complete.
- **Concurrent tests** (`tests/foundation.rs`) — the round-1 test
  `concurrent_loop_activate_serializes_via_fs2_lock` (lines 237-313)
  exercises fs2 lock + `--expect-revision` race. The round-2 test
  `concurrent_replan_and_task_start_preserves_plan_locked_invariant`
  (lines 356-491) exercises the TOCTOU closure re-check without
  `--expect-revision` (where revision is not sufficient by itself).
  Both tests assert on-disk end state, not just stdout. Meaningful.

Event-ordering spot-checks on the non-work-phase mutations:

- `loop activate/pause/resume/deactivate` all funnel through
  `src/cli/loop_/mod.rs::run_transition`. The Apply(wet) path at
  lines 103-107 builds the payload before calling `state::mutate`;
  the closure simply sets `s.loop_ = target`. Events appended before
  STATE persist by virtue of `state::mutate`'s invariant. No direct
  STATE write outside `state::mutate`.
- `outcome ratify` (`src/cli/outcome/ratify.rs:60-75`) — mutates STATE
  first via `state::mutate`, then writes OUTCOME.md with `atomic_write`.
  Comment at 54-59 documents the round-1 P1-4 decision (state is
  authoritative; OUTCOME.md is secondary). EVENTS row lands inside the
  closure.
- `close record-review` clean + dirty paths
  (`src/cli/close/record_review.rs:113-123, 183-211`) — both mutate
  STATE via `state::mutate` first; the dirty path writes the findings
  file outside the closure afterwards. Same pattern as outcome ratify.
- `replan record` (`src/cli/replan/record.rs:54-64`) — sole mutation
  point; closure applies all replan fields atomically (status flips,
  counter clear, triggered flag, plan unlock, phase set).

No findings for round 3.

## P0

None.

## P1

None.

## P2

None.

## P3

None in loop scope. Round-1 P3-1 (`dag.rs:51` expect comment) and
round-2 P3-1/P3-2/P3-3 remain the same latent observations and have
already been rejected under the "not in loop scope" rule (see
`docs/audits/round-1/decisions.md:38`, `docs/audits/round-2/decisions.md:19-21`);
no new P3 to surface.
