# Round 4 — correctness-invariants audit

HEAD: `703a171` (round-3 fixes: 0 P0 + 2 P1 + 2 P2 fixed (+ 2 rejected)).

Lens: runtime correctness — fs2 lock discipline, atomic-write durability,
EVENTS.jsonl append-order + monotonic seq, `--expect-revision` strict
equality, dirty-counter rules, verdict derivation ordering, panic hazards,
`CliError` completeness, concurrency test adequacy, and the round-3
`rewrite_status_to_ratified` regression.

## Summary

**No new findings.** Round-4 is a convergence round for this lens.

Scope walked (every mutating surface plus every read-only derivation that
feeds a gate):

- `state/mod.rs`, `state/fs_atomic.rs`, `state/events.rs`,
  `state/readiness.rs`, `state/schema.rs`
- `cli/outcome/{ratify,validate,check,emit,mod}.rs`
- `cli/plan/{check,choose_level,scaffold,dag,parsed,graph,waves,mod}.rs`
- `cli/task/{start,finish,next,lifecycle,packet,worker_packet,status_cmd,mod}.rs`
- `cli/review/{start,record,classify,plan_read,packet,status_cmd,mod}.rs`
- `cli/replan/{record,check,triggers,mod}.rs`
- `cli/loop_/{activate,pause,resume,deactivate,mod}.rs`
- `cli/close/{check,complete,record_review,closeout,mod}.rs`
- `cli/status/{mod,project,next_action}.rs`
- `cli/{init,hook,doctor,mod}.rs`, `bin/codex1.rs`
- `core/{error,envelope,mission,paths,config,mod}.rs`

### Invariants re-verified

1. **fs2 lock discipline.** Every state-mutating call path routes through
   `state::mutate` at `state/mod.rs:91-152`, which takes the exclusive
   `fs2` lock on `STATE.json.lock` before reading, mutating, or writing.
   Read-only callers go through `state::load` at `state/mod.rs:74-87`
   (shared lock, dropped before parse — correct). No side-channel writes
   to STATE.json exist outside `state::mutate` / `state::init_write`.

2. **Atomic write.** `state/fs_atomic.rs::atomic_write` uses
   `tempfile::NamedTempFile::new_in(dir)` → `write_all` → `sync_data` →
   `persist(target)` → best-effort parent-dir `sync_all`. This is applied
   to STATE.json (`state/mod.rs:145`), PLAN.yaml scaffold
   (`plan/scaffold.rs:77`), OUTCOME.md ratify (`outcome/ratify.rs:75`),
   CLOSEOUT.md (`close/complete.rs:82`), mission-close findings
   (`close/record_review.rs:211`), review findings copies
   (`review/record.rs:376`), plus the init templates
   (`init.rs:121,151`). All targets covered.

3. **EVENTS.jsonl append-only + monotonic seq, BEFORE STATE.** The round-1
   ordering fix is intact: `state/mod.rs:126-145` still appends the event
   (`events_cursor` bumped, then `append_event`) before `atomic_write` of
   STATE.json. `append_event` at `state/events.rs:41-47` uses
   `OpenOptions::new().create(true).append(true)` + `sync_data`, which is
   append-only by kernel contract. `seq = state.events_cursor` is
   incremented by `saturating_add(1)` once per mutation, matching the
   post-mutation `revision` bump 1-for-1.

4. **`--expect-revision` strict equality.** Every short-circuit honors it
   via `state::check_expected_revision`:
   - `task/start.rs:49` (idempotent), `:78` (dry-run)
   - `task/finish.rs:74` (dry-run)
   - `review/start.rs:66` (dry-run)
   - `review/record.rs:101` (dry-run)
   - `plan/check.rs:78` (idempotent + dry-run, shared branch)
   - `plan/choose_level.rs:53` (dry-run)
   - `plan/scaffold.rs:28-33` (dry-run, open-coded but equivalent — prior
     rounds classified the open-code as P3/REJECT per round-2 decisions)
   - `outcome/ratify.rs:34` (dry-run)
   - `close/complete.rs:53` (dry-run)
   - `close/record_review.rs:95` (clean dry-run), `:154` (dirty dry-run)
   - `loop_/mod.rs:75, 79, 92` (reject, no-op, apply dry-run)
   - `replan/record.rs:41-48` (dry-run, open-coded; same P3/REJECT class)
   Wet mutations enforce the check inside `state::mutate` at
   `state/mod.rs:114-122`.

5. **Dirty counter — only `accepted_current` bumps.** `review/record.rs::apply_record`
   at `:292` gates `apply_clean` / `apply_dirty` on
   `ReviewRecordCategory::AcceptedCurrent` — `late_same_boundary`,
   `stale_superseded`, `contaminated_after_terminal` never touch the
   counter. `apply_dirty` at `:346-361` bumps
   `consecutive_dirty_by_target[tid]`; `apply_clean` at `:302-325` resets
   each target to 0. Mission-close dirty path at
   `close/record_review.rs:192-197` keys the counter on the sentinel
   `MISSION_CLOSE_TARGET = "__mission_close__"` so it can never collide
   with a real `T<n>`. Round-1's `late_same_boundary` invariant test is
   still wired.

6. **Verdict derivation ordering matches handoff.** `state/readiness.rs::derive_verdict`
   at `:40-66`: Terminal → unratified outcome → unlocked plan →
   `replan.triggered` → blocking dirty review → tasks_complete branch →
   ContinueRequired. This is the frozen order. 11 unit tests in
   `state::readiness::tests` cover each arm plus the `stop_allowed`
   gating rules (all passing under `cargo test -p codex1 --lib`).

7. **Panic hazards.** `rg "\.unwrap\(\)|\.expect\(|panic!|unreachable!|todo!"`
   across `crates/codex1/src` returns only two non-test sites:
   - `cli/plan/dag.rs:51` `expect("indegree entry")` — trivially sound
     (the key was just inserted from the id set one line above).
   - `cli/plan/choose_level.rs:162` `expect("build_payload constructs a
     JSON object literal")` — sound because `json!({...})` always returns
     `Value::Object` per serde_json contract; comment pinned at `:161`.
   Both were explicitly REJECTed in round-1 decisions (P3-4: "comment
   only, not loop-scope"). Not re-raised.

8. **`CliError` completeness.** 18 canonical codes preserved at
   `core/error.rs:81-102`. Reserved variants (`ConfigMissing`,
   `NotImplemented`, `ReplanRequired`) still carry unit tests for
   envelope shape stability at `core/error.rs:181-253`. `Io`/`Json`/`Yaml`
   collapse to `PARSE_ERROR` with `ExitKind::Bug` per
   `core/error.rs:101,162`, covered by
   `io_error_maps_to_parse_error_and_bug_exit` unit test.

9. **Concurrency tests.** Two meaningful concurrency tests cover the
   module-level invariant bar:
   - `tests/foundation.rs::concurrent_loop_activate_serializes_via_fs2_lock`
     — asserts exactly-one success + REVISION_CONFLICT on the loser,
     final `revision=1`, EVENTS.jsonl has exactly one line with `seq=1`.
   - `tests/foundation.rs::concurrent_replan_and_task_start_preserves_plan_locked_invariant`
     — 4 iterations racing `task start` against `replan record`, asserts
     no `!plan.locked && task.T2.status == "in_progress"` shape.
   Round-3 REJECTed expanding to all four TOCTOU paths (P3-2) on the
   round-1 precedent of "one concurrent test per mutation-module
   invariant is the bar". Not re-raised.

### Round-4 regression checks

**`rewrite_status_to_ratified` (round-3 fix).** Verified stable across
idempotent replays. The implementation at
`cli/outcome/ratify.rs:105-154` builds output as
`"---\n" + new_front + "---\n" + body`. Because `split_frontmatter`
(`cli/outcome/validate.rs:154-174`) strips exactly one `\n` after the
opening fence and terminates `body` at the byte after the closing
fence's newline, re-parsing the output yields the identical
`(frontmatter_raw, body)` pair — replay N+1 is byte-identical to
replay N. The two tests added in round 3
(`ratify_preserves_closing_fence_without_blank_body_prefix`,
`ratify_is_file_level_idempotent_across_repeated_calls`) cover the two
historical failure shapes: the hand-written no-blank-line file and the
scaffolded template on repeat ratify. Both are structural (re-parseable
via `outcome check`, standalone `---` fence, no `---# OUTCOME` collapse),
which is the invariant the round-3 fix ships. Byte-for-byte equality is
implied by input-purity — no need to assert it explicitly. The YAML body
is preserved byte-for-byte except for the first top-level `status:` line
rewrite, whose indent + line-ending detection is covered at
`:112-118,156-164` (unit test `rewrites_status_line_preserving_body`).

**Round-1/2/3 fixes still intact:**

- EVENTS-before-STATE ordering: `state/mod.rs:126-145` (round-1 P1-1).
- Parent-dir fsync after `persist`: `state/fs_atomic.rs:33-35`
  (round-1 P1-2).
- OUTCOME.md written after state mutation on ratify:
  `cli/outcome/ratify.rs:60-75` (round-1 P1-4).
- `check_expected_revision` on every short-circuit: enumerated in item 4
  above (round-1 P1-5 + round-2 P2-1).
- `require_plan_locked` re-check inside closures:
  `cli/task/start.rs:111`, `cli/task/finish.rs:112`,
  `cli/review/start.rs:95`, `cli/review/record.rs:185` (round-2 P1-1).
- `plan check` clears `replan.triggered`: `cli/plan/check.rs:133-134`
  (round-2 e2e P0-1).

### Read-only modules cross-checked

`cli/replan/check.rs` (trigger probe) and `cli/status/mod.rs` +
`cli/status/project.rs` + `cli/status/next_action.rs` are pure
projections over `state::load` — no `state::mutate` calls, no duplicated
verdict logic (they route through `readiness::derive_verdict`). No
invariant concerns.

## P0

None.

## P1

None.

## P2

None.

## P3

None raised. The two known `.expect()` call sites in `cli/plan/dag.rs:51`
and `cli/plan/choose_level.rs:162` remain as prior rounds left them —
classified REJECT/P3 ("comment-only, out of loop scope") in round-1
decisions; re-raising would duplicate a closed finding. Open-coded
`--expect-revision` checks at `cli/plan/scaffold.rs:28-33` and
`cli/replan/record.rs:41-48` are equivalent to the helper call and were
explicitly REJECTed in round-2 (correctness P3-2) as style-only drift.
