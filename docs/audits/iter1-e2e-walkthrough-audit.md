# End-to-end walkthrough audit (iter 1)

Branch: main @ 64736506d1b2b61815be09546a9893ef43b286a6
Audited on: 2026-04-20 (UTC)
Build: `make install-local` → `cargo build --release` (17.05s from worktree)
Tests: 169 passed / 169 total (cargo test --release, 18 test binaries plus doc-tests)

## Summary

**PASS** on the functional walkthrough. All baseline findings F1..F7 are
verified fixed end-to-end against the running binary. The complete mission
drives from `codex1 init` through `codex1 close complete`; `task next`
and `status.next_action` agree at every captured phase; `CLOSEOUT.md`
lists T4 under Tasks; `state.tasks.T4` exists with `status: complete`;
the `review packet` envelope uses `proofs` (canonical name); and
`docs/cli-contract-schemas.md` documents the `review record` extras plus
the per-command relative path resolution rule.

Findings: **0 P0**, **1 P1**, **0 P2**.

One new **P1 regression** introduced by commit 6473650: `cargo clippy
--all-targets -- -D warnings` fails on `crates/codex1/tests/plan_scaffold.rs:38`
(`clippy::needless-raw-string-hashes`). The commit message explicitly
claims this gate is "clean"; it is not. `cargo clippy --lib --bins` is
still clean. Single-char fix, but today any CI job that runs
`--all-targets` against commit 6473650 will red-ball.

Baseline F1..F7 status: **all FIXED.**

## Baseline fix verification

- **F1** — FIXED. After T2/T3 finish (both AwaitingReview, rev 10),
  `task next` returned `run_review T4 [T2,T3]` **and**
  `status.next_action.kind == "run_review"` with `review_task_id: T4`,
  `ready_tasks: ["T4"]`. Both surfaces agree (walkthrough §10, step17/18).
  After T4 recorded clean (rev 12), `task next` returned
  `mission_close_review` and `status.next_action.kind` also
  `mission_close_review` (walkthrough §12, step22/23). Baseline showed
  `status` stuck on `run_wave W2 [T2,T3]` at rev 10 and `task next`
  returning stale `run_task T4` at rev 12; neither now occurs.

- **F2** — FIXED. After `review record T4 --clean` (rev 12),
  `status.review_required` is `[]` (step23). Remained `[]` at rev 13
  (step27) and rev 14 / terminal (step30). Baseline showed T4 stuck in
  `review_required` at every phase including terminal.

- **F3** — FIXED. After `review record T4 --clean` (rev 12),
  `codex1 task status T4 --mission demo` returned
  `{status: "complete", kind: "review", finished_at: "…"}` (step24).
  `state.tasks.T4` exists in STATE.json at rev 14 with
  `status: "complete"`. Implementation sits in
  `crates/codex1/src/cli/review/record.rs:286-328`
  (`apply_clean` → `mark_review_task_complete`).

- **F4** — FIXED. `codex1 review packet T4` envelope at rev 11 contains
  `proofs: ["PLANS/demo/specs/T2/PROOF.md", "PLANS/demo/specs/T3/PROOF.md"]`
  (step20). The legacy `target_proofs` field is gone. Additive fields
  (`target_specs`, `profiles`, `mission_id`, `reviewer_instructions`)
  stay and are declared additive in
  `docs/cli-contract-schemas.md:269-272`.

- **F5** — FIXED. `docs/cli-contract-schemas.md:274-291` documents
  `findings_file`, `replan_triggered`, and `warnings` as additive
  fields. Runtime envelope at rev 12 (step21) matches:
  `{review_task_id, verdict, category, reviewers, findings_file,
  replan_triggered, warnings}`.

- **F6** — FIXED. New section **"Per-command relative path resolution"**
  at `docs/cli-contract-schemas.md:83-98` pins:
  `task finish --proof` resolves against `mission_dir`;
  `review record --findings-file` and `close record-review
  --findings-file` resolve against CWD. Covers the exact rule the
  baseline audit flagged as undocumented.

- **F7** — FIXED. `PLANS/demo/CLOSEOUT.md` after `close complete`
  (step29) Tasks table lists T4 explicitly:
  ```
  | T4 | complete | — |
  ```
  Note the `—` in the Proof column is expected: review tasks don't
  produce proofs. All four task ids (T1..T4) appear in the Tasks
  table; T4 also appears in the Reviews table (correct double-book).

## Walkthrough transcript

### Step 1 — init
```json
{"ok": true, "mission_id": "demo", "revision": 0,
 "data": {"next_action": {"command": "$clarify", "kind": "clarify", ...},
          "created": {...}}}
```

### Step 2 — plan choose-level before ratify (F/P1-1 fix check)
```json
{"ok": false, "code": "OUTCOME_NOT_RATIFIED",
 "message": "OUTCOME.md is not ratified", "retryable": false}
```
Exit code 1. Gate fires.

### Step 3 — outcome check (after OUTCOME.md filled via heredoc)
```json
{"ok": true, "revision": 0,
 "data": {"ratifiable": true, "missing_fields": [], "placeholders": []}}
```

### Step 4 — outcome ratify
```json
{"ok": true, "revision": 1,
 "data": {"phase": "plan", "ratified_at": "2026-04-20T17:45:29.467966Z"}}
```

### Step 5 — plan choose-level (after ratify)
```json
{"ok": true, "revision": 2,
 "data": {"requested_level": "medium", "effective_level": "medium",
          "next_action": {"kind": "plan_scaffold",
                          "args": ["codex1","plan","scaffold","--level","medium"]}}}
```

### Step 6 — plan scaffold
```json
{"ok": true, "revision": 3,
 "data": {"level": "medium", "wrote": "PLANS/demo/PLAN.yaml",
          "specs_created": []}}
```
(PLAN.yaml overwritten with the 4-task diamond DAG via heredoc;
SPEC.md stubs written under specs/T{1..4}/.)

### Step 7 — plan check
```json
{"ok": true, "revision": 4,
 "data": {"tasks": 4, "review_tasks": 1, "hard_evidence": 0,
          "locked": true,
          "plan_hash": "sha256:88a1abee01d8c6f663da86103cce4ec2a24333e29b247f710472dcd44b9902de"}}
```

### Step 8 — plan waves
```json
{"ok": true, "revision": 4,
 "data": {"all_tasks_complete": false, "current_ready_wave": "W1",
          "waves": [
            {"wave_id": "W1", "tasks": ["T1"], "parallel_safe": true, "blockers": []},
            {"wave_id": "W2", "tasks": ["T2","T3"], "parallel_safe": true, "blockers": []},
            {"wave_id": "W3", "tasks": ["T4"], "parallel_safe": true, "blockers": []}
          ]}}
```

### Step 9 — task next (fresh plan)
```json
{"ok": true, "revision": 4,
 "data": {"next": {"kind": "run_task", "task_id": "T1", "task_kind": "code"}}}
```

### Step 10 — task lifecycle T1..T3
- `task start T1` → rev 5, status `in_progress`.
- `task finish T1 --proof specs/T1/PROOF.md` → rev 6, status `complete`
  (T1 has no review_target so transitions straight to complete).
- `task next` → rev 6, `run_wave W2 [T2,T3]`.
- `task start T2` / `task finish T2` → rev 7/8, T2 → `awaiting_review`.
- `task start T3` / `task finish T3` → rev 9/10, T3 → `awaiting_review`.

### Step 11 — F1 evidence: task next and status after T2/T3 finish (rev 10)
```json
// task next
{"next": {"kind": "run_review", "targets": ["T2","T3"], "task_id": "T4"}}

// status
{"next_action": {"command": "$review-loop", "kind": "run_review",
                 "review_task_id": "T4", "targets": ["T2","T3"]},
 "ready_tasks": ["T4"],
 "review_required": [{"task_id": "T4", "targets": ["T2","T3"]}],
 "verdict": "continue_required"}
```
Both surfaces agree. `ready_tasks` is `[T4]` (not `[T2,T3]`). Baseline F1 fixed.

### Step 12 — F4 evidence: review packet shape
```json
{"ok": true, "revision": 11,
 "data": {"task_id": "T4", "review_profile": "code_bug_correctness",
          "targets": ["T2","T3"],
          "proofs": ["PLANS/demo/specs/T2/PROOF.md",
                     "PLANS/demo/specs/T3/PROOF.md"],
          // additive fields per contract §269-272:
          "profiles": ["code_bug_correctness"],
          "target_specs": [{task_id, spec_path, spec_excerpt}, ...],
          "diffs": [{"path": "src/T2/**"}, {"path": "src/T3/**"}],
          "mission_summary": "...",
          "mission_id": "demo",
          "reviewer_instructions": "You are a Codex1 reviewer. ..."}}
```
Field `proofs` is present; `target_proofs` is absent. F4 fixed.

### Step 13 — F5 evidence: review record --clean envelope
```json
{"ok": true, "revision": 12,
 "data": {"review_task_id": "T4", "verdict": "clean",
          "category": "accepted_current",
          "reviewers": ["e2e-iter1-auditor"],
          // extras documented at cli-contract-schemas.md:274-291:
          "findings_file": null,
          "replan_triggered": false,
          "warnings": []}}
```
Guaranteed + additive fields match contract exactly. F5 fixed.

### Step 14 — F1/F2/F3 evidence: task next + status + task status T4 (rev 12)
```json
// task next — NOT "run_task T4" (baseline bug); now routes to mission-close.
{"next": {"kind": "mission_close_review",
          "reason": "all tasks complete or superseded"}}

// status — review_required is empty, ready_tasks is empty.
{"next_action": {"command": "$review-loop (mission-close)",
                 "kind": "mission_close_review", ...},
 "ready_tasks": [],
 "review_required": [],
 "verdict": "ready_for_mission_close_review"}

// task status T4 — COMPLETE (baseline reported "ready").
{"task_id": "T4", "kind": "review", "status": "complete",
 "depends_on": ["T2","T3"],
 "deps_status": {"T2": "complete", "T3": "complete"},
 "finished_at": "2026-04-20T17:46:56.123399Z"}
```
F1 (agreement), F2 (cleared), F3 (TaskRecord present with status=complete) all fixed.

### Step 15 — close check vs status agreement
- Rev 12 (after T4 clean): `status.verdict` and `close check.verdict`
  both `ready_for_mission_close_review`; `close_ready` false; blocker
  `CLOSE_NOT_READY: mission-close review has not started`.
- Rev 13 (after `close record-review --clean --reviewers mc-iter1`):
  both `mission_close_review_passed`; `close_ready: true`; blockers `[]`.
- Rev 14 (after `close complete`): both `terminal_complete`;
  `close_ready: false` (verdict != mission_close_review_passed, per
  contract §192). No regression — contract-correct.

### Step 16 — close complete
```json
{"ok": true, "revision": 14,
 "data": {"closeout_path": "/private/tmp/codex1-iter1-e2e/PLANS/demo/CLOSEOUT.md",
          "terminal_at": "2026-04-20T17:47:20.440031Z",
          "mission_id": "demo", "dry_run": false}}
```

### Step 17 — F7 evidence: CLOSEOUT.md Tasks table
```markdown
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
| T4 | clean | e2e-iter1-auditor | — |
| MC | clean | — | — |
```
T4 (kind=review) is listed in Tasks. F7 fixed.

### Step 18 — terminal idempotency
```json
{"ok": false, "code": "TERMINAL_ALREADY_COMPLETE",
 "message": "Mission is already terminal (closed at 2026-04-20T17:47:20.440031Z)",
 "hint": "Start a new mission; a terminal mission cannot be reopened.",
 "retryable": false,
 "context": {"closed_at": "2026-04-20T17:47:20.440031Z"}}
```
Envelope matches contract `TERMINAL_ALREADY_COMPLETE`.

### Step 19 — EVENTS.jsonl
14 lines; seq 1..14 strictly increasing; timestamps monotonic. Kinds
captured in order: `outcome.ratified`, `plan.choose_level`,
`plan.scaffold`, `plan.checked`, `task.started/finished` ×3,
`review.started`, `review.recorded.clean`, `close.review.clean`,
`close.complete`.

### Step 20 — Ralph hook manual test
```shell
# allow path
$ PATH=/tmp/ralph-test-pass-iter1:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
exit: 0

# block path
$ PATH=/tmp/ralph-test-block-iter1:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
ralph-stop-hook: blocking Stop - reason=active_loop
ralph-stop-hook: mock block
exit: 2
```
Exits 0 on `stop.allow: true`, 2 on `stop.allow: false`. Contract-correct.

## status ↔ close check agreement table

| Phase / revision | loop.active | loop.paused | verdict | close_ready | close check.ready | stop.allow | stop.reason |
|---|---|---|---|---|---|---|---|
| after init (rev 0) | false | false | needs_user | false | false | true | idle |
| after ratify (rev 1) | false | false | needs_user | false | false | true | idle |
| after T3 finish (rev 10) | false | false | continue_required | false | false | true | idle |
| after T4 clean (rev 12) | false | false | ready_for_mission_close_review | false | false | true | idle |
| after MC review (rev 13) | false | false | mission_close_review_passed | true | true | true | idle |
| after close complete (rev 14) | false | false | terminal_complete | false | false | true | terminal |

`close_ready` and `close check.ready` agree at 6/6 captured phases.

## Findings

### P1-1 — cargo clippy --all-targets fails on test file introduced by 6473650

- Command: `cargo clippy --all-targets -- -D warnings` (from worktree).
- File:line: `crates/codex1/tests/plan_scaffold.rs:38`.
- Lint: `clippy::needless-raw-string-hashes`.
- Observed:
  ```
  error: unnecessary hashes around raw string literal
    --> crates/codex1/tests/plan_scaffold.rs:38:19
     |
  38 |     let outcome = r#"---
     | ^^^^^^^
     = help: for further information visit
             https://rust-lang.github.io/rust-clippy/rust-1.94.0/index.html#needless_raw_string_hashes
     = note: `-D clippy::needless-raw-string-hashes` implied by `-D warnings`

  error: could not compile `codex1` (test "plan_scaffold") due to 1 previous error
  ```
- Expected: `cargo clippy --all-targets -- -D warnings` clean.
- Commit-message claim (64736506, §"Test suite"):
  > "cargo fmt + cargo clippy --all-targets -- -D warnings clean"
  This claim is false; the test target introduced in this commit uses
  `r#"..."#` around a literal that contains no `"`/`#`, which trips
  `needless_raw_string_hashes` on rust 1.94. `cargo clippy --lib --bins`
  is still clean; only the `--all-targets` gate fails.
- Severity rationale (P1): the commit message asserts a gate is green
  that isn't. Any CI job running `cargo clippy --all-targets -- -D
  warnings` against 6473650 will red-ball. Fix is trivial (strip the
  `#` hashes and the matching closing `"#`).

## Clean checks

- [x] `codex1 --help` lists every documented command (init, doctor,
  hook, outcome, plan, task, review, replan, loop, close, status, help
  — 11 subcommands + help).
- [x] `codex1 --json doctor` returns `ok: true` without auth
  (`required: false`, `warnings: []`).
- [x] Full mission reaches `terminal_complete` (status at rev 14).
- [x] `CLOSEOUT.md` is written and cites every task id including T4
  (T1..T4 in Tasks table; T4, MC in Reviews table). F7 fix visible.
- [x] `EVENTS.jsonl` is append-only and monotonic (seq 1..14; timestamps
  chronological).
- [x] `codex1 status` and `codex1 close check` agree on `close_ready` /
  `verdict` at 6/6 captured phases.
- [x] Ralph hook shell exits 0 when `stop.allow: true`, 2 when false
  (manual test with mocked `codex1` on PATH).
- [x] `cargo fmt --check` clean.
- [x] `cargo test --release` — 169 passed / 169 total (0 failed, 0
  ignored).
- [ ] `cargo clippy --all-targets -- -D warnings` — **FAILS** on
  `tests/plan_scaffold.rs:38` (see P1-1).
- [x] `cargo clippy --lib --bins -- -D warnings` clean.

All functional clean checks pass; a single CI lint gate fails on a
test-only target, tracked as P1-1.

## Pre-existing non-regression notes (not new findings)

- `review packet` `mission_summary` still contains the literal YAML
  block-scalar indicator `"|\n"` as a prefix (e.g.
  `"|\nMission reaches ..."`). This was present in the baseline audit
  (§24) and is not regressed by 6473650. Cosmetic; not scored.
- `review start T4 --mission demo` mutates revision (rev 10→11). Not a
  finding; matches baseline behaviour and is used to pin
  `boundary_revision`.
