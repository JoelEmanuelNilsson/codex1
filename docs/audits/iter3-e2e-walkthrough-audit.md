# End-to-end walkthrough audit (iter 3)

Branch: main @ 958d2f1
Audited: 2026-04-20 (UTC)
Build: cargo build --release (install-local completed in 17.51s)
Tests: 167 passed / 2 failed / 169 total
fmt: PASS | clippy: **FAIL** | tests: **FAIL**

## Summary

**FAIL** — baseline F1..F8 surface-symptom behaviours are fixed and the
happy-path walkthrough reaches `verdict: terminal_complete` with
`CLOSEOUT.md` listing every DAG task (T1, T2, T3, T4). The F8
regression test (partial completion) passes: with only T1 finished,
`status.verdict == "continue_required"`, `status.close_ready == false`,
and `close check.ready == false`, all three.

However, the F8 fix in 958d2f1 introduced **two new P1 regressions**
against the task spec's build-gate (§4: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --release`):

1. **Clippy fails** at `crates/codex1/src/cli/plan/check.rs:109`
   (`clippy::assigning_clones` — the F8 fix wrote
   `s.plan.task_ids = task_ids_to_record.clone();` which should be
   `s.plan.task_ids.clone_from(&task_ids_to_record);`). Same class of
   regression the iter 1 e2e reviewer flagged on 6473650 and that
   iter1-fix 271b2fc resolved; the F8 fix re-introduced it.
2. **Tests fail** — two tests in `crates/codex1/tests/status.rs`
   (`all_tasks_complete_reports_ready_for_mission_close_review` and
   `mission_close_review_passed_reports_close_next_action`) set
   `state.plan.locked = true` and insert T1..T4 into `state.tasks`
   directly, but do not populate the new `state.plan.task_ids` field.
   Both now fail with
   `assertion left == right failed: "continue_required" !=
   "ready_for_mission_close_review"` (and the analogous
   `mission_close_review_passed` failure).

Also identified one P2 (backward-compat): the idempotent short-circuit
in `plan/check.rs:62–84` returns before the mutation closure, so an
already-locked STATE.json with empty `plan.task_ids` (produced by any
`plan check` run at 6473650 or 271b2fc, before the F8 field existed)
will never backfill `task_ids` via `plan check`. Combined with the new
`tasks_complete` predicate, such a mission gets permanently stuck at
`verdict: continue_required` with `next_action.kind: "blocked"` /
reason "No ready wave derivable" even after every task is finished.
Recovery is possible (edit PLAN.yaml to invalidate the hash, or
trigger replan) but there is no automated affordance.

Baseline F1..F8 status: **F1–F7 FIXED (iter 1), F8 root cause FIXED as
a feature (iter 2) but the fix introduces 2 new P1 + 1 P2 (this
report)**.

Findings: **0 P0**, **2 P1**, **1 P2**.

## Walkthrough transcript

Full happy-path and F8 regression captures follow. Every call was made
with `--json` and every envelope is quoted below verbatim (trimmed only
to the contract-critical fields where `...` is shown).

### 1. Install

```shell
$ cd /Users/joel/codex1/.claude/worktrees/agent-a69ac9f7 && time make install-local
   <snip: 100+ dependency compile lines>
   Compiling codex1 v0.1.0 (/Users/joel/codex1/.claude/worktrees/agent-a69ac9f7/crates/codex1)
    Finished `release` profile [optimized] target(s) in 17.51s
cp target/release/codex1 /Users/joel/.local/bin/codex1
Installed codex1 to /Users/joel/.local/bin/codex1
```

### 2. Verify from /tmp

```shell
$ cd /tmp && command -v codex1
/Users/joel/.local/bin/codex1

$ codex1 --help | head
Drives a mission through clarify → plan → execute → review-loop → close. ...
Usage: codex1 [OPTIONS] <COMMAND>
Commands:
  init  doctor  hook  outcome  plan  task  review  replan  loop  close  status  help
```

### 3. Doctor

```shell
$ cd /tmp && codex1 --json doctor
{
  "ok": true,
  "data": {
    "auth": { "required": false, "notes": "Codex1 is a local mission harness; no auth is required by default." },
    "config": { "exists": false, "path": "/Users/joel/.codex1/config.toml" },
    "cwd": "/private/tmp",
    "install": {
      "codex1_on_path": "/Users/joel/.local/bin/codex1",
      "home_local_bin": "/Users/joel/.local/bin",
      "home_local_bin_writable": true
    },
    "version": "0.1.0",
    "warnings": []
  }
}
```

### 4. Fresh init (happy-path mission)

```shell
$ cd /tmp && rm -rf codex1-iter3-happy && mkdir codex1-iter3-happy && cd codex1-iter3-happy
$ codex1 --json init --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 0,
  "data": {
    "created": { "mission_dir": ".../PLANS/demo", "outcome": ".../OUTCOME.md", ... },
    "next_action": { "command": "$clarify", "kind": "clarify",
                     "hint": "Fill in OUTCOME.md, then run `codex1 outcome ratify`." }
  }
}
```

### 5. OUTCOME.md filled + ratify (happy path)

Filled every `[codex1-fill:…]` marker with a minimal-but-valid payload
(mission `demo`, 4-task DAG description).

```shell
$ codex1 --json outcome check --mission demo
{ "ok": true, "revision": 0,
  "data": { "missing_fields": [], "placeholders": [], "ratifiable": true } }

$ codex1 --json outcome ratify --mission demo
{ "ok": true, "revision": 1,
  "data": { "mission_id": "demo", "phase": "plan", "ratified_at": "2026-04-20T18:30:05.843899Z" } }
```

### 6. Plan choose-level (medium), scaffold, fill, check

```shell
$ codex1 --json plan choose-level --level medium --mission demo
{ "ok": true, "revision": 2,
  "data": { "effective_level": "medium", "requested_level": "medium",
            "next_action": { "kind": "plan_scaffold", "args": ["codex1","plan","scaffold","--level","medium"] } } }

$ codex1 --json plan scaffold --level medium --mission demo
{ "ok": true, "revision": 3,
  "data": { "level": "medium", "specs_created": [], "wrote": "PLANS/demo/PLAN.yaml" } }
```

Wrote a 4-task DAG (T1 → T2,T3 → T4-review-of-[T2,T3]) and created
`specs/T{1,2,3,4}/SPEC.md`.

```shell
$ codex1 --json plan check --mission demo
{ "ok": true, "revision": 4,
  "data": { "tasks": 4, "review_tasks": 1, "hard_evidence": 0, "locked": true,
            "plan_hash": "sha256:06ca12d26bc61ac212b4851272162a2cfcf6fb9d9d7212e10ac4eb5e95bd757d" } }
```

**Verified F8 feature landed correctly on this mission:**

```shell
$ python3 -c "import json; print('task_ids:', json.load(open('PLANS/demo/STATE.json'))['plan']['task_ids'])"
task_ids: ['T1', 'T2', 'T3', 'T4']
```

### 7. plan waves

```shell
$ codex1 --json plan waves --mission demo
{
  "ok": true, "revision": 4,
  "data": {
    "all_tasks_complete": false,
    "current_ready_wave": "W1",
    "waves": [
      { "wave_id": "W1", "tasks": ["T1"],      "parallel_safe": true, "blockers": [] },
      { "wave_id": "W2", "tasks": ["T2","T3"], "parallel_safe": true, "blockers": [] },
      { "wave_id": "W3", "tasks": ["T4"],      "parallel_safe": true, "blockers": [] }
    ]
  }
}
```

### 8. Task lifecycle — W1 (T1), W2 (T2 + T3), W3 (T4 review)

```shell
$ codex1 --json task start T1 --mission demo
{ "ok": true, "revision": 5,
  "data": { "task_id": "T1", "status": "in_progress", "started_at": "2026-04-20T18:30:26.431781Z", "idempotent": false } }

$ echo "#p" > PLANS/demo/specs/T1/PROOF.md
$ codex1 --json task finish T1 --proof specs/T1/PROOF.md --mission demo
{ "ok": true, "revision": 6,
  "data": { "task_id": "T1", "status": "complete", "finished_at": "...", "proof_path": "specs/T1/PROOF.md" } }

$ codex1 --json task start T2 ... finish T2 --proof specs/T2/PROOF.md ...
{ "ok": true, "revision": 8,
  "data": { "task_id": "T2", "status": "awaiting_review", ... } }

$ codex1 --json task start T3 ... finish T3 --proof specs/T3/PROOF.md ...
{ "ok": true, "revision": 10,
  "data": { "task_id": "T3", "status": "awaiting_review", ... } }

$ codex1 --json task start T4 --mission demo
{ "ok": true, "revision": 11,
  "data": { "task_id": "T4", "status": "in_progress", ... } }
```

### 9. Review T4 (self-review gate for T2+T3)

```shell
$ codex1 --json review start T4 --mission demo
{ "ok": true, "revision": 12,
  "data": { "review_task_id": "T4", "targets": ["T2","T3"], "verdict": "pending", "boundary_revision": 12 } }

$ codex1 --json review record T4 --clean --reviewers code-reviewer --mission demo
{ "ok": true, "revision": 13,
  "data": { "review_task_id": "T4", "verdict": "clean",
            "category": "accepted_current", "reviewers": ["code-reviewer"],
            "findings_file": null, "replan_triggered": false, "warnings": [] } }
```

Clean review on T4 auto-finishes T4 (T4.status → complete) and clears
T2/T3 into complete.

### 10. status reflects "all tasks done → ready for mission close"

```shell
$ codex1 --json status --mission demo
{ "ok": true, "revision": 13,
  "data": {
    "verdict": "ready_for_mission_close_review",
    "close_ready": false,
    "next_action": { "kind": "mission_close_review", "command": "$review-loop (mission-close)",
                     "hint": "All tasks complete; run the mission-close review." },
    "outcome_ratified": true, "plan_locked": true, "phase": "execute",
    "ready_tasks": [], "review_required": [], "replan_required": false,
    "stop": { "allow": true, "reason": "idle", "message": "Loop is inactive; stop is allowed." },
    ...
  } }
```

### 11. close check (pre-review) → record-review → close check (ready) → close complete

```shell
$ codex1 --json close check --mission demo
{ "ok": true, "revision": 13,
  "data": { "verdict": "ready_for_mission_close_review", "ready": false,
            "blockers": [ { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" } ] } }

$ codex1 --json close record-review --clean --reviewers code-reviewer,architect-reviewer --mission demo
{ "ok": true, "revision": 14,
  "data": { "review_state": "passed", "verdict": "clean",
            "reviewers": ["code-reviewer","architect-reviewer"], "findings_file": null,
            "replan_triggered": false, "consecutive_dirty": 0, "dry_run": false } }

$ codex1 --json close check --mission demo
{ "ok": true, "revision": 14,
  "data": { "verdict": "mission_close_review_passed", "ready": true, "blockers": [] } }

$ codex1 --json close complete --mission demo
{ "ok": true, "revision": 15,
  "data": { "mission_id": "demo", "closeout_path": ".../CLOSEOUT.md",
            "terminal_at": "2026-04-20T18:31:07.79111Z", "dry_run": false } }
```

### 12. Final status & CLOSEOUT.md

```shell
$ codex1 --json status --mission demo
{ "ok": true, "revision": 15,
  "data": { "verdict": "terminal_complete", "close_ready": false,
            "next_action": { "kind": "closed", "hint": "Mission is terminal." },
            "phase": "terminal",
            "stop": { "allow": true, "reason": "terminal", "message": "Mission is terminal; stop is allowed." },
            ... } }
```

`CLOSEOUT.md` (verbatim):

```markdown
# CLOSEOUT — demo

**Terminal at:** 2026-04-20T18:31:07.79111Z
**Final revision:** 15
**Planning level:** medium

## Outcome

A demo mission with T1 seed, T2/T3 parallel branches, and T4 review
of T2+T3 - reaching terminal_complete with CLOSEOUT.md listing all
four tasks.

## Tasks

| ID | Status | Proof |
|---|---|---|
| T1 | complete | specs/T1/PROOF.md |
| T2 | complete | specs/T2/PROOF.md |
| T3 | complete | specs/T3/PROOF.md |
| T4 | complete | — |

## Reviews

| Review ID | Verdict | Reviewers | Findings |
|---|---|---|---|
| T4 | clean | code-reviewer | — |
| MC | clean | — | — |

## Mission-close review

Clean on the first round.
```

Every DAG node (T1, T2, T3, T4) is present. Baseline F7 remains fixed.

## F8 regression check — partial-completion test

Spec from the task prompt:
`rm -rf /tmp/codex1-iter3-f8` → init → fill OUTCOME → ratify →
choose-level medium → scaffold → write 4-task PLAN + 4 SPECs →
plan check → start/finish T1 (only) → assert
`status.verdict == "continue_required"`,
`status.close_ready == false`,
`close check.ready == false`.

### Setup (identical to spec)

```shell
$ cd /tmp && rm -rf codex1-iter3-f8 && mkdir codex1-iter3-f8 && cd codex1-iter3-f8
$ codex1 --json init --mission f8    # revision 0
$ # …filled OUTCOME.md, ratified, choose-level medium, scaffold…
$ # …wrote PLAN.yaml with T1,T2,T3,T4 (T4 review of [T2,T3]) + 4 SPEC files…
$ codex1 --json plan check --mission f8
{ "ok": true, "revision": 4,
  "data": { "tasks": 4, "review_tasks": 1, "locked": true,
            "plan_hash": "sha256:ade9f2fc5025639f21ce97720184a14012b011afa1e623ed7d5173e5a6e256b4", ... } }

$ python3 -c "import json; print('task_ids:', json.load(open('PLANS/f8/STATE.json'))['plan']['task_ids'])"
task_ids: ['T1', 'T2', 'T3', 'T4']   # ← F8 fix populates plan.task_ids
```

### Partial completion: start + finish T1 only

```shell
$ codex1 --json task start T1 --mission f8
{ "ok": true, "revision": 5, "data": { "task_id": "T1", "status": "in_progress", ... } }

$ echo "# proof" > PLANS/f8/specs/T1/PROOF.md
$ codex1 --json task finish T1 --proof specs/T1/PROOF.md --mission f8
{ "ok": true, "revision": 6, "data": { "task_id": "T1", "status": "complete", ... } }
```

### Assertion 1 — status.verdict / status.close_ready

```shell
$ codex1 --json status --mission f8
{
  "ok": true, "revision": 6,
  "data": {
    "verdict": "continue_required",   ← ★ not ready_for_mission_close_review
    "close_ready": false,              ← ★
    "next_action": { "kind": "run_wave", "wave_id": "W2", "tasks": ["T2","T3"],
                     "parallel_safe": true, "hint": "Run wave W2 with $execute." },
    "outcome_ratified": true, "plan_locked": true, "phase": "execute",
    "ready_tasks": ["T2","T3"], "review_required": [], "replan_required": false,
    "parallel_blockers": [], "parallel_safe": true,
    "stop": { "allow": true, "reason": "idle", "message": "Loop is inactive; stop is allowed." },
    "loop": { "active": false, "mode": "none", "paused": false }
  }
}
```

PASS — `status.verdict == "continue_required"` and
`status.close_ready == false`, exactly per spec.

### Assertion 2 — close check.ready / close check.verdict

```shell
$ codex1 --json close check --mission f8
{
  "ok": true, "revision": 6,
  "data": {
    "verdict": "continue_required",    ← ★
    "ready": false,                    ← ★
    "blockers": [ { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" } ]
  }
}
```

PASS — `close check.ready == false` and
`close check.verdict == "continue_required"`. Agreement with status.

### F8 verdict

F8 regression check **PASSES**: the 958d2f1 fix correctly gates
mission-close on the full DAG task-id list, and `tasks_complete`
refuses to flip the verdict when any DAG node has no `Complete`/
`Superseded` entry in `state.tasks`. Both commands (`status` and
`close check`) agree.

## Findings

### F9 (P1) — clippy regression in F8 fix: `assigning_clones`

**Location:** `crates/codex1/src/cli/plan/check.rs:109`
**Introduced in:** 958d2f1
**Build-gate violated:** task spec §4,
`cargo clippy --all-targets -- -D warnings`

```shell
$ cargo clippy --all-targets -- -D warnings
error: assigning the result of `Clone::clone()` may be inefficient
   --> crates/codex1/src/cli/plan/check.rs:109:13
    |
109 |             s.plan.task_ids = task_ids_to_record.clone();
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: use `clone_from()`: `s.plan.task_ids.clone_from(&task_ids_to_record)`
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.94.0/index.html#assigning_clones
    = note: `-D clippy::assigning-clones` implied by `-D warnings`
error: could not compile `codex1` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: could not compile `codex1` (lib test) due to 1 previous error
```

This is the same class of regression the iter 1 e2e reviewer flagged
on 6473650 (clippy failed before shipping) and that iter1-fix 271b2fc
resolved. The F8 fix at 958d2f1 introduced a new clippy warning via
`s.plan.task_ids = task_ids_to_record.clone();`; the clippy suggestion
is a mechanical one-line rewrite. The author likely did not run
`cargo clippy --all-targets -- -D warnings` locally before committing
(same pattern as iter 1).

**Suggested fix location (non-prescriptive):** Replace
`s.plan.task_ids = task_ids_to_record.clone();`
with
`s.plan.task_ids.clone_from(&task_ids_to_record);`
at `crates/codex1/src/cli/plan/check.rs:109`. Zero semantic change.

### F10 (P1) — `cargo test --release` regression: two status tests unset

**Location:** `crates/codex1/tests/status.rs:329–391` —
`all_tasks_complete_reports_ready_for_mission_close_review` and
`mission_close_review_passed_reports_close_next_action`
**Introduced in:** 958d2f1
**Build-gate violated:** task spec §4, `cargo test --release`

```shell
$ cargo test --release --no-fail-fast 2>&1 | grep -E "^test result|FAILED"
...
test result: FAILED. 12 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

failures:
    all_tasks_complete_reports_ready_for_mission_close_review
    mission_close_review_passed_reports_close_next_action

---- all_tasks_complete_reports_ready_for_mission_close_review stdout ----
thread '...' panicked at crates/codex1/tests/status.rs:355:5:
assertion `left == right` failed
  left: String("continue_required")
 right: "ready_for_mission_close_review"

---- mission_close_review_passed_reports_close_next_action stdout ----
thread '...' panicked at crates/codex1/tests/status.rs:388:5:
assertion `left == right` failed
  left: String("continue_required")
 right: "mission_close_review_passed"
```

Root cause: both tests set `state.plan.locked = true` and insert all
four task records via `state.tasks.insert(...)`, but do **not**
populate `state.plan.task_ids`. The F8 fix added the requirement that
`tasks_complete` iterate `state.plan.task_ids` rather than
`state.tasks.values()` — so with `task_ids == []` the predicate now
returns `false` (fail-closed), the verdict stays
`continue_required`, and the assertions on
`ready_for_mission_close_review` / `mission_close_review_passed` fail.

Final release test tally:
**167 passed / 2 failed / 169 total** (not 170 as claimed in the
958d2f1 commit message — the new F8 fixture was added inside the
already-existing `status_agrees_with_readiness_helpers_for_all_fixtures`
parameterized test and does not change the `#[test]` count).

The iter 2 report recorded 169/169 passing; these 2 tests passed at
271b2fc and now fail at 958d2f1.

**Suggested fix location (non-prescriptive):** In
`crates/codex1/tests/status.rs`, populate
`state.plan.task_ids = vec!["T1".into(), "T2".into(), "T3".into(), "T4".into()]`
(or equivalent) in both test bodies. The commit message for 958d2f1
states the fix updated `close.rs builder auto-computes from the tasks
added; status_close_agreement.rs fixtures set explicitly`, but the
`status.rs` fixtures — which build `MissionState` by hand without a
builder — were missed.

### F11 (P2) — idempotent `plan check` short-circuit blocks `task_ids` backfill

**Location:** `crates/codex1/src/cli/plan/check.rs:62–84`
**Introduced in:** 958d2f1 (pre-existing idempotency path, now
load-bearing because `task_ids` must be populated)
**Scope:** backward-compat for any STATE.json produced at 6473650 or
271b2fc with `plan.locked == true` — the PlanState struct now has
`#[serde(default)]` on `task_ids`, so those files deserialize with
`task_ids == []` even though the plan is locked.

Reproduction:

```shell
$ cd /tmp && mkdir codex1-iter3-backcompat && cd codex1-iter3-backcompat
$ codex1 --json init --mission bc                 # revision 0
$ # …fill OUTCOME, ratify, choose-level, scaffold, fill PLAN, plan check…
$ python3 -c "import json; s=json.load(open('PLANS/bc/STATE.json')); print('task_ids:', s['plan']['task_ids'])"
task_ids: ['T1', 'T2', 'T3', 'T4']

# Simulate a pre-F8 STATE.json by clearing task_ids while keeping locked+hash.
$ python3 -c "
import json
p='PLANS/bc/STATE.json'; s=json.load(open(p)); s['plan']['task_ids']=[]
open(p,'w').write(json.dumps(s,indent=2))
print('after:', json.load(open(p))['plan']['task_ids'])"
after: []

# Re-run plan check (same PLAN.yaml, same hash). The idempotent
# short-circuit triggers; task_ids is NOT backfilled.
$ codex1 --json plan check --mission bc
{ "ok": true, "revision": 4,
  "data": { "tasks": 4, "locked": true,
            "plan_hash": "sha256:40fada42...", ... } }

$ python3 -c "import json; print('task_ids:', json.load(open('PLANS/bc/STATE.json'))['plan']['task_ids'])"
task_ids: []                                       # ← NOT backfilled

# Finish every task. The mission is still stuck at continue_required
# because tasks_complete() returns false on empty task_ids.
$ codex1 --json task start T1 ; codex1 --json task finish T1 --proof specs/T1/PROOF.md
$ # …same for T2, T3, T4…
$ codex1 --json status --mission bc
{
  "ok": true, "revision": 12,
  "data": {
    "verdict": "continue_required",
    "close_ready": false,
    "next_action": { "kind": "blocked",
                     "reason": "No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json." },
    "ready_tasks": [], "parallel_safe": false, "parallel_blockers": [],
    ...
  }
}
```

The mission is permanently stuck. Re-running `plan check` is the
natural recovery call and it looks successful (`locked: true` in the
envelope), but the envelope's `locked: true` hides the fact that
`task_ids` was never set. The `next_action.reason` of "No ready wave
derivable — PLAN.yaml may be missing, empty, or inconsistent with
STATE.json" hints at the inconsistency but does not point the user
at `plan.task_ids`.

Fail-closed behaviour is defensible — better stuck than a P0 false
positive — but this is a user-visible trap on any upgrade-in-place.
Since `plan.task_ids` only landed four commits ago (958d2f1), any
STATE.json from a mid-audit walkthrough at 6473650 / 271b2fc is
affected.

**Workaround:** Edit PLAN.yaml to flip its SHA (e.g. add/remove
whitespace) and re-run `plan check`; the hash mismatch forces the
mutation closure to run and backfill `task_ids`. Alternatively,
trigger a replan (which clears `plan.locked`) then `plan check` again.

**Suggested fix location (non-prescriptive):** In the short-circuit
at `crates/codex1/src/cli/plan/check.rs:66–84`, also treat "plan
locked but `plan.task_ids` is empty" as a condition that must run
the mutation closure — e.g. expand the gate to
`already_locked_same = current.plan.locked
  && current.plan.hash.as_deref() == Some(hash.as_str())
  && !current.plan.task_ids.is_empty();`.
This backfills on any re-run and is safe (the mutation is a pure
overwrite, no other state changes).

## Clean checks

- [x] `codex1 --help` lists every documented command (11 subcommands:
  init, doctor, hook, outcome, plan, task, review, replan, loop,
  close, status).
- [x] `codex1 --json doctor` returns `ok: true` with `auth.required:
  false` and no warnings. §3.
- [x] Every CLI call in the happy path returns a well-formed JSON
  envelope with contract keys (`ok`, `mission_id`, `revision`, `data`
  on success; `ok`, `code`, `message`, `retryable` on error). §4–§11.
- [x] `plan check` populates `state.plan.task_ids` with the full DAG
  task-id list at lock time. §6, F8 §Setup.
- [x] **F8 root cause FIXED** — with only T1 finished out of
  {T1,T2,T3,T4}, `status.verdict == "continue_required"`,
  `status.close_ready == false`, `close check.ready == false`,
  `close check.verdict == "continue_required"`. §F8 Assertions 1 & 2.
- [x] **Baseline F1 fixed** — at revision 10 (T2,T3 in
  awaiting_review), `status.ready_tasks == []`, `review_required`
  lists the T4 review, `next_action.kind == "run_wave"` with
  `tasks == ["T4"]`. (iter 1 fix stable at 958d2f1.)
- [x] **Baseline F2 fixed** — after `review record T4 --clean`,
  `status.review_required == []`. §10.
- [x] **Baseline F3 fixed (surface symptom)** — after clean review of
  T4, T4 auto-finishes (`status: complete`) and T2/T3 resolve to
  `complete`. §9 and CLOSEOUT.md.
- [x] **Baseline F4 fixed** — `review packet` emits the `proofs` field
  (regression test `review.rs` passes).
- [x] **Baseline F5 additive fields documented** — `review record`
  envelope carries `category`, `replan_triggered`, `warnings`,
  `findings_file`. §9.
- [x] **Baseline F6 doc updated** — relative mission-dir-relative
  proof paths work (`--proof specs/T1/PROOF.md`). §8.
- [x] **Baseline F7 fixed** — CLOSEOUT.md Tasks table lists T1, T2,
  T3, T4 (including the T4 review row). §12.
- [x] `status.close_ready` and `close check.ready` agree at every
  captured phase — pre-review (§10/§11: both false),
  post-record-review (§11: both true), terminal (§12: `close_ready:
  false`, next_action "closed").
- [x] Mission reaches `verdict: terminal_complete`. §12.
- [x] `CLOSEOUT.md` lists every DAG task (T1, T2, T3, T4). §12.
- [x] Ralph hook exits 0 on `stop.allow: true` and exits 2 on
  `stop.allow: false` (with reason / message on stderr). §Ralph hook.
- [x] `cargo fmt --check` clean.
- [ ] `cargo clippy --all-targets -- -D warnings` clean — **FAIL**
  (F9).
- [ ] `cargo test --release` passes — **FAIL**, 167/169 pass (F10).
- [x] EVENTS.jsonl append-only, `seq` monotonically increasing,
  timestamps chronological (15 events in happy-path). §EVENTS excerpt.

## Ralph hook

Driven with a PATH-scoped mock `codex1` that returns a hand-built
status envelope.

```shell
# allow=true case
$ cat /tmp/ralph-mock-allow-true/codex1
#!/usr/bin/env bash
if [ "$1" = "status" ]; then
  cat <<'JSON'
{"ok":true,"mission_id":"mock","revision":1,"data":{"stop":{"allow":true,"reason":"idle","message":"Loop inactive"},"verdict":"needs_user"}}
JSON
  exit 0
fi
echo "mock"
$ PATH="/tmp/ralph-mock-allow-true:$PATH" bash scripts/ralph-stop-hook.sh < /dev/null
$ echo "exit=$?"
exit=0

# allow=false case
$ cat /tmp/ralph-mock-allow-false/codex1
#!/usr/bin/env bash
if [ "$1" = "status" ]; then
  cat <<'JSON'
{"ok":true,"mission_id":"mock","revision":1,"data":{"stop":{"allow":false,"reason":"active_loop","message":"Loop active; stop blocked"},"verdict":"continue_required"}}
JSON
  exit 0
fi
echo "mock"
$ PATH="/tmp/ralph-mock-allow-false:$PATH" bash scripts/ralph-stop-hook.sh < /dev/null
ralph-stop-hook: blocking Stop - reason=active_loop
ralph-stop-hook: Loop active; stop blocked
$ echo "exit=$?"
exit=2
```

Both paths behave as contract: exit 0 on allow, exit 2 with stderr
log on block. PASS.

## EVENTS.jsonl excerpt (happy-path)

```jsonl
{"seq":1,"at":"2026-04-20T18:30:05.856767Z","kind":"outcome.ratified",...}
{"seq":2,"at":"2026-04-20T18:30:09.454307Z","kind":"plan.choose_level",...}
{"seq":3,"at":"2026-04-20T18:30:09.46714Z","kind":"plan.scaffold",...}
{"seq":4,"at":"2026-04-20T18:30:16.606987Z","kind":"plan.checked","payload":{"effective_level":"medium","hard_evidence":0,"plan_hash":"sha256:06ca12d2...","requested_level":"medium","review_tasks":1,"tasks":4}}
{"seq":5,"at":"...","kind":"task.started","payload":{"task_id":"T1",...}}
{"seq":6,"at":"...","kind":"task.finished","payload":{"task_id":"T1","next_status":"complete","proof_path":"specs/T1/PROOF.md",...}}
{"seq":7,"at":"...","kind":"task.started","payload":{"task_id":"T2",...}}
{"seq":8,"at":"...","kind":"task.finished","payload":{"task_id":"T2","next_status":"awaiting_review",...}}
{"seq":9,"at":"...","kind":"task.started","payload":{"task_id":"T3",...}}
{"seq":10,"at":"...","kind":"task.finished","payload":{"task_id":"T3","next_status":"awaiting_review",...}}
{"seq":11,"at":"...","kind":"task.started","payload":{"task_id":"T4",...}}
{"seq":12,"at":"...","kind":"review.started","payload":{"review_task_id":"T4","targets":["T2","T3"]}}
{"seq":13,"at":"...","kind":"review.recorded.clean","payload":{"review_task_id":"T4","reviewers":["code-reviewer"],"targets":["T2","T3"],"verdict":"clean"}}
{"seq":14,"at":"...","kind":"close.review.clean","payload":{"reviewers":["code-reviewer","architect-reviewer"],"verdict":"clean"}}
{"seq":15,"at":"2026-04-20T18:31:07.796954Z","kind":"close.complete","payload":{"terminal_at":"2026-04-20T18:31:07.79111Z"}}
```

15 events, monotonic `seq`, chronological timestamps, append-only.

## Conclusion

FAIL — 0 P0, 2 P1 (F9 clippy, F10 tests), 1 P2 (F11 idempotent
short-circuit blocks backfill). F8 root-cause fix is semantically
correct and verified end-to-end (partial-completion cannot flip the
verdict), but the fix introduces a clippy regression and leaves two
pre-existing status tests failing — both of which fail the task spec
§4 build-gate. F1–F8 surface symptoms are all fixed. Target 0/0/0 is
not met this iteration.
