# Round 5 — correctness-invariants audit

HEAD audited: `b08d461` (round-4 fixes committed).

Reviewer 5/6, lens: runtime correctness invariants across
`crates/codex1/src/`.

## Summary

No new P0/P1/P2 findings. All eight checklist invariants hold on this HEAD;
the round-4 round-4 spot check (`cli/review/packet.rs`) is verified clean on
both axes — no panic path on malformed YAML, and sibling parity confirmed
byte-for-byte with `cli/task/worker_packet.rs` (the sibling the round-4
decision explicitly names). A sharper divergence in a third sibling
(`cli/close/closeout.rs::extract_frontmatter`) is noted for completeness
but does not meet the strict P0/P1/P2 bar — it's P3 territory under
precedent (round-2 P3-1, round-3 P3-2), and round-5 is not P3-scope.

Per-invariant walk below.

### 1. fs2 lock discipline on every STATE.json mutation

`state::mutate` (`crates/codex1/src/state/mod.rs:91-152`) acquires an
exclusive `fs2` lock at line 101, reads STATE.json at line 109, runs
the closure, appends the event, writes STATE.json, and drops the lock
at line 146. Every writer enters through this helper — confirmed via
`Grep 'state::mutate\|fs2\|lock_exclusive\|lock_shared'`: the only
`lock_exclusive` / `lock_shared` call sites are `state::mod.rs`
itself. `load` (`state/mod.rs:74`) uses a shared lock. PASS.

### 2. Atomic write for STATE.json / PLAN.yaml / CLOSEOUT.md / OUTCOME.md / findings

`fs_atomic::atomic_write` (`state/fs_atomic.rs:22-37`) uses
`NamedTempFile::new_in(parent_dir)` → `sync_data()` → `persist(target)`
→ parent-dir `sync_all()`. Round-1 added the parent-dir fsync; still
present. All callers route through `atomic_write`:

- STATE.json: `state::mutate:145`, `state::init_write:175`
- PLAN.yaml: `cli/plan/scaffold.rs:77`, `cli/init.rs:151`
- CLOSEOUT.md: `cli/close/complete.rs:82`
- OUTCOME.md: `cli/outcome/ratify.rs:75`, `cli/init.rs:121`
- Findings files: `cli/review/record.rs:376`,
  `cli/close/record_review.rs:211`
- EVENTS.jsonl init: `state::init_write:177`

PASS.

### 3. EVENTS.jsonl append-only, monotonic seq, before STATE.json persist

`state::mutate:142-145` bumps `state.events_cursor`, constructs an
`Event` with that cursor as `seq`, appends it via `append_event`, and
only then writes STATE.json. `append_event` (`state/events.rs:41-47`)
opens with `create(true).append(true)`, writes one line, and
`sync_data`s. Ordering rationale is documented at
`state/mod.rs:126-141` (round-1 fix). `Event::new` (`state/events.rs:26-37`)
takes the caller's `seq`, and the mutation closure is the only producer.
PASS.

### 4. `--expect-revision` strict equality on all short-circuits

Every mutating command honors `check_expected_revision` on every path
that can return without calling `state::mutate`. Traced via `Grep
'check_expected_revision|expect_revision'`:

- `task/start.rs:49,78,101`: idempotent + dry-run + mutate
- `task/finish.rs:74,100`: dry-run + mutate
- `review/start.rs:66,85`: dry-run + mutate (round-2 P2-1 fix)
- `review/record.rs:101,129,171`: dry-run + stale-event mutate + main mutate
- `outcome/ratify.rs:34,62`: dry-run + mutate
- `close/complete.rs:53,66`: dry-run + mutate
- `close/record_review.rs:95,116,154,185`: clean dry + clean mutate + dirty dry + dirty mutate
- `plan/check.rs:78,110`: idempotent/dry-run short-circuit + mutate
- `plan/choose_level.rs:53,71`: dry-run + mutate
- `loop_/mod.rs:75,79,92,104`: reject-before-error + noop + dry-run + mutate

Two open-coded `expected != state.revision` comparisons remain
(`plan/scaffold.rs:27-34`, `replan/record.rs:41-48`) — already
rejected as round-2 P3-2 on the ground that the inlined check is
semantically identical to the helper. Not re-raising.

PASS.

### 5. Dirty counter rules

Two sites, both strict `saturating_add(1)` per target:

- Task reviews (`cli/review/record.rs:346-361`): only
  `AcceptedCurrent` records touch the counter. `LateSameBoundary`,
  `StaleSuperseded`, and `ContaminatedAfterTerminal` are explicitly
  skipped by the `matches!(category, AcceptedCurrent)` guard at
  line 292 (round-1 P2-1 test `late_same_boundary_does_not_bump_or_reset_dirty_counter`
  still pins this). Clean resets to 0 per target. Dirty streak
  threshold `DIRTY_STREAK_THRESHOLD = 6` (`record.rs:29`).
- Mission-close review (`cli/close/record_review.rs:192-207`):
  `counter.saturating_add(1)` under `MISSION_CLOSE_TARGET`,
  `DIRTY_REPLAN_THRESHOLD = 6` (`record_review.rs:26`). Guards `hit
  && !state.replan.triggered` before flipping `triggered` so a
  second hit doesn't overwrite the first reason.

PASS.

### 6. Verdict derivation ordering

`state::readiness::derive_verdict` (`state/readiness.rs:40-66`):

1. `close.terminal_at` → `TerminalComplete`
2. `!outcome.ratified` → `NeedsUser`
3. `!plan.locked` → `NeedsUser`
4. `replan.triggered` → `Blocked`
5. `has_blocking_dirty` → `Blocked`
6. `tasks_complete`: `NotStarted → ReadyForMissionCloseReview`,
   `Open → MissionCloseReviewOpen`, `Passed →
   MissionCloseReviewPassed`
7. else → `ContinueRequired`

Matches the "Pinned semantics — Verdict derivation" contract. 11
round-1 unit tests cover each arm plus `close_ready_is_only_true_for_passed`,
`stop_allowed_when_loop_inactive_or_paused`,
`tasks_complete_requires_dag_and_records`.

PASS.

### 7. `unwrap()` / `expect()` / `panic!()` justified and user-unreachable

Full `Grep '\.unwrap\(\)|\.expect\(|panic!'` over
`crates/codex1/src`. Production call sites (excluding `#[cfg(test)]`):

- `cli/plan/choose_level.rs:162` — `.expect("build_payload
  constructs a JSON object literal")`. The comment at 159-161
  proves the invariant: `json!({…})` always produces `Object`.
  Unreachable. Round-1 drive-by comment.
- `cli/plan/dag.rs:51` — `.expect("indegree entry")`. Protected by
  graph shape built two lines above. Round-2 P3-1 rejected.
- `cli/outcome/ratify.rs:174,183` — test-only.
- `cli/outcome/validate.rs:349` — test-only.
- `cli/plan/dag.rs:109,126,139` — test-only (in
  `mod tests { #[test] fn … }`).
- `core/error.rs:199,213,227,246` — test-only.

No production `.unwrap()` on user-supplied data.

PASS.

### 8. `CliError` completeness

`core/error.rs:24-76` defines 18 typed variants plus three
passthroughs (`Io`, `Json`, `Yaml`). `code()` (80-103), `retryable()`
(106-108), `hint()` (111-143), `context()` (146-157), `kind()`
(160-165), `to_envelope()` (168-176) are exhaustive by `match`.
`retryable` is `true` only for `RevisionConflict`; `kind` routes
`Io|Json|Yaml` to `ExitKind::Bug`, everything else to
`HandledError`. Reserved variants (`ConfigMissing`, `NotImplemented`,
`ReplanRequired`) are unit-tested at `core/error.rs:191-230` plus the
shape-stability test at 239-253 (round-1 + round-3 additions).

PASS.

### Round-4 spot check — `cli/review/packet.rs` post-serde_yaml swap

Task: verify (a) no panic path on malformed YAML, (b) behavior
matches sibling paths (`task/worker_packet.rs`, `close/closeout.rs`).

(a) **No panic path.**
`cli/review/packet.rs:148-158`: `read_interpreted_destination`
threads `.ok()?` through every fallible step — `fs::read_to_string`,
`extract_frontmatter`, `serde_yaml::from_str`, and the
`get → as_str → trim` chain. Call site at
`cli/review/packet.rs:58`: `.unwrap_or_default()`. No `.unwrap()`,
`.expect()`, or `panic!()` on the parse path. Malformed YAML
degrades to empty-string `mission_summary`, matching the documented
tolerance model at the inline doc-comment (140-147).

(b) **Sibling parity.**
`cli/review/packet.rs::extract_frontmatter` (164-177) is
byte-for-byte identical to `cli/task/worker_packet.rs::extract_frontmatter`
(72-86) — the sibling the round-4 decision names as the mirror. Both
strip the UTF-8 BOM, accept LF and CRLF fences, and fall back to the
verbatim trimmed string if no `---\n`/`---\r\n` prefix is found.
The parse path (`serde_yaml::from_str` → `get → as_str → trim`) is
identical in both files. Behaviour converges.

`close/closeout.rs::extract_frontmatter` (144-150) is stricter —
no BOM strip, no verbatim fallback, combined LF/CRLF match in one
chain — but its caller (`read_interpreted_destination` at 136-142)
also returns `None` on miss, and `render` (line 34) falls back to a
placeholder string. The behaviour is visible-equivalent at the
renderer level. This is a style drift, not a correctness drift —
under round-5's strict criteria it is P3 territory (round-2 P3-1
"open-code vs helper" and round-3 P3-2 "sibling convergence style"
were both rejected as non-loop-scope). Not raised.

Round-4 fix: VERIFIED CLEAN on both axes.

## P0

None.

## P1

None.

## P2

None.

## P3

None raised. Scope restricted to P0/P1/P2 per task instructions.
Observed style drifts (`plan/scaffold.rs` + `replan/record.rs`
open-coded `expect_revision`, `close/closeout.rs::extract_frontmatter`
stricter than its siblings) are already on the precedent list and
would duplicate rejected findings from prior rounds.
