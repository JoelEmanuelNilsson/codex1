# End-to-end walkthrough audit (iter 2)

Branch: main @ 271b2fc
Audited: 2026-04-20 (UTC)
Build: cargo build --release (in 19.96s)
Tests: 169 passed / 169 total
fmt: PASS | clippy: PASS | tests: PASS

## Summary

**FAIL** — the iter 1 fixes for F1, F2, F4, F5, F6, F7 are verified live,
and the visible happy-path walkthrough still reaches
`verdict: terminal_complete` with `CLOSEOUT.md` listing every DAG node
(T1..T4). However, the iter 1 fix for baseline F3 was **partial**: the
root cause that the baseline reviewer flagged at
`crates/codex1/src/state/readiness.rs:74-82` — `tasks_complete`
iterates `state.tasks.values()` and silently ignores plan tasks that
have never been recorded — remains live and is now proven to allow
a **P0 data-integrity violation**: a mission can reach
`verdict: terminal_complete` with only 1 of 4 planned tasks done.
Reproduction transcript under `F8` below.

Findings: **1 P0**, **0 P1**, **0 P2**.

## Walkthrough transcript

### 1. Install

```shell
$ cd /Users/joel/codex1/.claude/worktrees/agent-a8b6e479 && time make install-local
   <snip: 60+ dependency compile lines>
   Compiling codex1 v0.1.0 (/Users/joel/codex1/.claude/worktrees/agent-a8b6e479/crates/codex1)
    Finished `release` profile [optimized] target(s) in 19.96s
cp target/release/codex1 /Users/joel/.local/bin/codex1
Installed codex1 to /Users/joel/.local/bin/codex1
```

### 2. Verify from /tmp

```shell
$ cd /tmp && command -v codex1
/Users/joel/.local/bin/codex1

$ codex1 --help
Drives a mission through clarify → plan → execute → review-loop → close. ...
Commands:
  init  doctor  hook  outcome  plan  task  review  replan  loop  close  status  help
```

### 3. Doctor

```shell
$ cd /tmp && codex1 --json doctor
{
  "ok": true,
  "data": {
    "auth": { "notes": "Codex1 is a local mission harness; no auth is required by default.", "required": false },
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

### 4. Init

```shell
$ cd /tmp && rm -rf codex1-iter2-e2e && mkdir codex1-iter2-e2e && cd codex1-iter2-e2e
$ codex1 --json init --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 0,
  "data": {
    "created": { "events": "…/EVENTS.jsonl", "mission_dir": "…/PLANS/demo", "outcome": "…/OUTCOME.md", "plan": "…/PLAN.yaml", "reviews_dir": "…/reviews", "specs_dir": "…/specs", "state": "…/STATE.json" },
    "next_action": { "command": "$clarify", "hint": "Fill in OUTCOME.md, then run `codex1 outcome ratify`.", "kind": "clarify" }
  }
}
```

### 5. Plan choose-level before ratify (expect OUTCOME_NOT_RATIFIED)

```shell
$ codex1 --json plan choose-level --mission demo --level medium
{
  "ok": false,
  "code": "OUTCOME_NOT_RATIFIED",
  "message": "OUTCOME.md is not ratified",
  "retryable": false
}
# exit 1
```

Contract gate fires. Good (matches iter 1 fix for CLI contract P1-1).

### 6. Outcome check after filling OUTCOME.md

```shell
$ codex1 --json outcome check --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 0,
  "data": { "missing_fields": [], "placeholders": [], "ratifiable": true }
}
```

### 7. Outcome ratify

```shell
$ codex1 --json outcome ratify --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 1,
  "data": { "mission_id": "demo", "phase": "plan", "ratified_at": "2026-04-20T18:03:12.211527Z" }
}
```

### 8. Plan choose-level (medium)

```shell
$ codex1 --json plan choose-level --mission demo --level medium
{
  "ok": true, "mission_id": "demo", "revision": 2,
  "data": {
    "effective_level": "medium",
    "next_action": { "args": ["codex1","plan","scaffold","--level","medium"], "kind": "plan_scaffold" },
    "requested_level": "medium"
  }
}
```

### 9. Plan scaffold

```shell
$ codex1 --json plan scaffold --mission demo --level medium
{
  "ok": true, "mission_id": "demo", "revision": 3,
  "data": { "level": "medium", "specs_created": [], "wrote": "PLANS/demo/PLAN.yaml" }
}
```

PLAN.yaml was overwritten with a 4-task DAG (T1 root, T2/T3 parallel
deps on T1, T4 review targeting [T2,T3]) and SPEC.md files were written
under `PLANS/demo/specs/T{1..4}/`.

### 10. Plan check

```shell
$ codex1 --json plan check --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 4,
  "data": {
    "hard_evidence": 0,
    "locked": true,
    "plan_hash": "sha256:70fe5a6524cc3ae879d3595820749b98c2b5526919b294f79cff4172b7fb8d59",
    "review_tasks": 1,
    "tasks": 4
  }
}
```

### 11. Plan waves

```shell
$ codex1 --json plan waves --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 4,
  "data": {
    "all_tasks_complete": false,
    "current_ready_wave": "W1",
    "waves": [
      { "blockers": [], "parallel_safe": true, "tasks": ["T1"], "wave_id": "W1" },
      { "blockers": [], "parallel_safe": true, "tasks": ["T2","T3"], "wave_id": "W2" },
      { "blockers": [], "parallel_safe": true, "tasks": ["T4"], "wave_id": "W3" }
    ]
  }
}
```

### 12. Task next (revision 4; plan locked, no tasks run)

```shell
$ codex1 --json task next --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 4,
  "data": { "next": { "kind": "run_task", "task_id": "T1", "task_kind": "code" } }
}

$ codex1 --json status --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 4,
  "data": {
    "close_ready": false,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": { "command": "$execute", "kind": "run_task", "task_id": "T1", "task_kind": "code" },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": true,
    "phase": "execute",
    "plan_locked": true,
    "ready_tasks": ["T1"],
    "replan_required": false,
    "review_required": [],
    "stop": { "allow": true, "message": "Loop is inactive; stop is allowed.", "reason": "idle" },
    "verdict": "continue_required"
  }
}
```

Agreement: both → `run_task T1 kind=code`. `ready_tasks: [T1]`. Good.

### 13. Task start T1

```shell
$ codex1 --json task start T1 --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 5,
  "data": { "idempotent": false, "started_at": "2026-04-20T18:04:25.872609Z", "status": "in_progress", "task_id": "T1" }
}
```

### 14. Task finish T1

```shell
$ echo "## Proof" > PLANS/demo/specs/T1/PROOF.md
$ codex1 --json task finish T1 --mission demo --proof specs/T1/PROOF.md
{
  "ok": true, "mission_id": "demo", "revision": 6,
  "data": { "finished_at": "2026-04-20T18:04:31.733634Z", "proof_path": "specs/T1/PROOF.md", "status": "complete", "task_id": "T1" }
}
```

T1 has no review target → transitions straight to `complete`. The
relative `specs/T1/PROOF.md` form works (F6 documentation fix
from iter 1 is validated — resolution rule is now on record).

### 15. Task next after T1 complete (revision 6) — **NEW DIVERGENCE**

```shell
$ codex1 --json task next --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 6,
  "data": {
    "next": {
      "kind": "run_wave",
      "parallel_safe": true,
      "tasks": ["T2","T3"],
      "wave_id": "W2"
    }
  }
}

$ codex1 --json status --mission demo
{
  "ok": true, "mission_id": "demo", "revision": 6,
  "data": {
    "close_ready": false,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": {
      "command": "$review-loop (mission-close)",
      "hint": "All tasks complete; run the mission-close review.",
      "kind": "mission_close_review"
    },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": true,
    "phase": "execute",
    "plan_locked": true,
    "ready_tasks": ["T2","T3"],
    "replan_required": false,
    "review_required": [],
    "stop": { "allow": true, "message": "Loop is inactive; stop is allowed.", "reason": "idle" },
    "verdict": "ready_for_mission_close_review"
  }
}
```

`task next` correctly routes to `run_wave W2 [T2,T3]`. `status`
disagrees: `verdict: ready_for_mission_close_review`, `next_action.kind:
mission_close_review`. The contract's "single source of truth" is
lying. See F8.

`task status T1` / T2 / T4 at revision 6:

```shell
$ codex1 --json task status T1 --mission demo
{ "status": "complete", "kind": "code", "depends_on": [], "deps_status": {}, … }

$ codex1 --json task status T2 --mission demo
{ "status": "ready", "kind": "code", "depends_on": ["T1"], "deps_status": {"T1": "complete"}, "task_id": "T2" }

$ codex1 --json task status T4 --mission demo
{ "status": "pending", "kind": "review", "depends_on": ["T2","T3"], "deps_status": {"T2": "pending", "T3": "pending"}, "task_id": "T4" }
```

`task status T2` knows T2 is ready; `task status T4` knows it is still
pending — yet `status` says the mission is ready to close-review. See
F8 for root cause and reproduction.

### 16. Task start T2 / finish T2

```shell
$ codex1 --json task start T2 --mission demo
{ "ok": true, "revision": 7, "data": { "idempotent": false, "started_at": "2026-04-20T18:05:01.592128Z", "status": "in_progress", "task_id": "T2" } }

$ echo "## Proof" > PLANS/demo/specs/T2/PROOF.md
$ codex1 --json task finish T2 --mission demo --proof specs/T2/PROOF.md
{ "ok": true, "revision": 8, "data": { "finished_at": "2026-04-20T18:05:06.09925Z", "proof_path": "specs/T2/PROOF.md", "status": "awaiting_review", "task_id": "T2" } }
```

T2 transitions to `awaiting_review` because T4 targets T2. Good.

### 17. Task start T3 / finish T3

```shell
$ codex1 --json task start T3 --mission demo
{ "ok": true, "revision": 9, "data": { "idempotent": false, "started_at": "2026-04-20T18:05:09.879913Z", "status": "in_progress", "task_id": "T3" } }

$ echo "## Proof" > PLANS/demo/specs/T3/PROOF.md
$ codex1 --json task finish T3 --mission demo --proof specs/T3/PROOF.md
{ "ok": true, "revision": 10, "data": { "finished_at": "2026-04-20T18:05:13.290243Z", "proof_path": "specs/T3/PROOF.md", "status": "awaiting_review", "task_id": "T3" } }
```

### 18. Task next after T3 awaiting_review (F1 check-point)

```shell
$ codex1 --json task next --mission demo
{
  "ok": true, "revision": 10,
  "data": { "next": { "kind": "run_review", "targets": ["T2","T3"], "task_id": "T4" } }
}

$ codex1 --json status --mission demo
{
  "ok": true, "revision": 10,
  "data": {
    "next_action": { "command": "$review-loop", "kind": "run_review", "review_task_id": "T4", "targets": ["T2","T3"] },
    "ready_tasks": ["T4"],
    "review_required": [ { "targets": ["T2","T3"], "task_id": "T4" } ],
    "verdict": "continue_required",
    …
  }
}
```

Both → `run_review T4 targets=[T2,T3]`. `ready_tasks: [T4]` (not
[T2,T3]). F1 fix is **verified live** — non-review AwaitingReview
tasks are correctly excluded from ready_tasks.

### 19. Review start T4

```shell
$ codex1 --json review start T4 --mission demo
{
  "ok": true, "revision": 11,
  "data": { "boundary_revision": 11, "review_task_id": "T4", "targets": ["T2","T3"], "verdict": "pending" }
}
```

### 20. Review packet T4 (F4 check-point)

```shell
$ codex1 --json review packet T4 --mission demo
{
  "ok": true, "revision": 11,
  "data": {
    "diffs": [],
    "mission_id": "demo",
    "mission_summary": "|\nA four-task DAG (T1 -> {T2,T3} -> T4 review) reaches terminal_complete\nend to end with every CLI call returning a contract-structured envelope.",
    "profiles": ["code_bug_correctness"],
    "proofs": ["PLANS/demo/specs/T2/PROOF.md", "PLANS/demo/specs/T3/PROOF.md"],
    "review_profile": "code_bug_correctness",
    "reviewer_instructions": "…",
    "target_specs": [
      { "spec_excerpt": "# T2 - Parallel branch A\n…", "spec_path": "specs/T2/SPEC.md", "task_id": "T2" },
      { "spec_excerpt": "# T3 - Parallel branch B\n…", "spec_path": "specs/T3/SPEC.md", "task_id": "T3" }
    ],
    "targets": ["T2","T3"],
    "task_id": "T4"
  }
}
```

Field `proofs` is present (not `target_proofs`). F4 fix **verified
live**. Additive extras (`profiles`, `target_specs`, `mission_id`,
`reviewer_instructions`) remain — documented per iter 1 fix.

### 21. Review record T4 --clean (F3 check-point)

```shell
$ codex1 --json review record T4 --clean --reviewers r1 --mission demo
{
  "ok": true, "revision": 12,
  "data": {
    "category": "accepted_current",
    "findings_file": null,
    "replan_triggered": false,
    "review_task_id": "T4",
    "reviewers": ["r1"],
    "verdict": "clean",
    "warnings": []
  }
}
```

`task status T4` after record-clean:

```shell
$ codex1 --json task status T4 --mission demo
{
  "ok": true, "revision": 12,
  "data": {
    "depends_on": ["T2","T3"],
    "deps_status": { "T2": "complete", "T3": "complete" },
    "finished_at": "2026-04-20T18:05:28.466572Z",
    "kind": "review",
    "status": "complete",
    "task_id": "T4"
  }
}
```

T4 is now `complete` in `state.tasks`. F3 fix for the review-task
TaskRecord linchpin **verified live**. (The deeper root cause survives
— see F8.)

STATE.json excerpt (revision 12):

```json
"tasks": {
  "T1": { "status": "complete", … },
  "T2": { "status": "complete", … },
  "T3": { "status": "complete", … },
  "T4": { "status": "complete", "finished_at": "2026-04-20T18:05:28.466572Z", "superseded_by": null }
},
"reviews": {
  "T4": { "task_id": "T4", "verdict": "clean", "reviewers": ["r1"], "category": "accepted_current", "boundary_revision": 11, … }
}
```

### 22. Task next after review clean (revision 12)

```shell
$ codex1 --json task next --mission demo
{
  "ok": true, "revision": 12,
  "data": { "next": { "kind": "mission_close_review", "reason": "all tasks complete or superseded" } }
}

$ codex1 --json status --mission demo
{
  "ok": true, "revision": 12,
  "data": {
    "close_ready": false,
    "next_action": { "command": "$review-loop (mission-close)", "hint": "All tasks complete; run the mission-close review.", "kind": "mission_close_review" },
    "ready_tasks": [],
    "review_required": [],
    "verdict": "ready_for_mission_close_review",
    …
  }
}
```

Both → `mission_close_review`. `ready_tasks: []`. `review_required: []`
(T4 is already recorded clean — F2 fix **verified live**). Good.

### 23. Close check before mission-close review

```shell
$ codex1 --json close check --mission demo
{
  "ok": true, "revision": 12,
  "data": {
    "blockers": [ { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" } ],
    "ready": false,
    "verdict": "ready_for_mission_close_review"
  }
}
```

`close check.ready` == `status.close_ready` == `false`. Agreed.

### 24. Close record-review --clean

```shell
$ codex1 --json close record-review --clean --reviewers mc --mission demo
{
  "ok": true, "revision": 13,
  "data": {
    "consecutive_dirty": 0,
    "dry_run": false,
    "findings_file": null,
    "replan_triggered": false,
    "review_state": "passed",
    "reviewers": ["mc"],
    "verdict": "clean"
  }
}
```

### 25. Close check after mission-close review

```shell
$ codex1 --json close check --mission demo
{
  "ok": true, "revision": 13,
  "data": { "blockers": [], "ready": true, "verdict": "mission_close_review_passed" }
}

$ codex1 --json status --mission demo
{
  "ok": true, "revision": 13,
  "data": {
    "close_ready": true,
    "next_action": { "command": "codex1 close complete", "hint": "Mission-close review passed; finalize close.", "kind": "close" },
    "ready_tasks": [],
    "review_required": [],
    "verdict": "mission_close_review_passed",
    …
  }
}
```

`close check.ready == status.close_ready == true`. Agreed.

### 26. Close complete

```shell
$ codex1 --json close complete --mission demo
{
  "ok": true, "revision": 14,
  "data": {
    "closeout_path": "/private/tmp/codex1-iter2-e2e/PLANS/demo/CLOSEOUT.md",
    "dry_run": false,
    "mission_id": "demo",
    "terminal_at": "2026-04-20T18:05:58.870278Z"
  }
}
```

### 27. Terminal status

```shell
$ codex1 --json status --mission demo
{
  "ok": true, "revision": 14,
  "data": {
    "close_ready": false,
    "next_action": { "hint": "Mission is terminal.", "kind": "closed" },
    "phase": "terminal",
    "verdict": "terminal_complete",
    "stop": { "allow": true, "message": "Mission is terminal; stop is allowed.", "reason": "terminal" },
    …
  }
}
```

### 28. Close complete idempotency

```shell
$ codex1 --json close complete --mission demo
{
  "ok": false,
  "code": "TERMINAL_ALREADY_COMPLETE",
  "message": "Mission is already terminal (closed at 2026-04-20T18:05:58.870278Z)",
  "hint": "Start a new mission; a terminal mission cannot be reopened.",
  "retryable": false,
  "context": { "closed_at": "2026-04-20T18:05:58.870278Z" }
}
# exit 1
```

### 29. CLOSEOUT.md (F7 check-point)

```markdown
# CLOSEOUT — demo

**Terminal at:** 2026-04-20T18:05:58.870278Z
**Final revision:** 14
**Planning level:** medium

## Outcome

A four-task DAG (T1 -> {T2,T3} -> T4 review) reaches terminal_complete
end to end with every CLI call returning a contract-structured envelope.

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
| T4 | clean | r1 | — |
| MC | clean | — | — |

## Mission-close review

Clean on the first round.
```

T4 is now in the Tasks table. F7 fix **verified live**.

### 30. EVENTS.jsonl — append-only, monotonic

```text
seq 1 outcome.ratified           2026-04-20T18:03:12.226154Z
seq 2 plan.choose_level          2026-04-20T18:03:16.417442Z
seq 3 plan.scaffold              2026-04-20T18:03:21.666132Z
seq 4 plan.checked               2026-04-20T18:04:09.481718Z
seq 5 task.started (T1)          2026-04-20T18:04:25.881813Z
seq 6 task.finished (T1)         2026-04-20T18:04:31.743746Z
seq 7 task.started (T2)          2026-04-20T18:05:01.617889Z
seq 8 task.finished (T2)         2026-04-20T18:05:06.104416Z
seq 9 task.started (T3)          2026-04-20T18:05:09.885988Z
seq 10 task.finished (T3)        2026-04-20T18:05:13.296768Z
seq 11 review.started (T4)       2026-04-20T18:05:23.137265Z
seq 12 review.recorded.clean     2026-04-20T18:05:28.475313Z
seq 13 close.review.clean        2026-04-20T18:05:51.416652Z
seq 14 close.complete            2026-04-20T18:05:58.877086Z
```

14 events; seq strictly increasing; timestamps chronological. Good.

### 31. Ralph hook manual test

```shell
$ mkdir -p /tmp/ralph-iter2-pass
$ cat > /tmp/ralph-iter2-pass/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":true,"reason":"idle","message":"mock idle"}}}'
EOF
$ chmod +x /tmp/ralph-iter2-pass/codex1

$ PATH=/tmp/ralph-iter2-pass:/usr/bin:/bin bash /Users/joel/codex1/scripts/ralph-stop-hook.sh </dev/null
exit=0

$ mkdir -p /tmp/ralph-iter2-block
$ cat > /tmp/ralph-iter2-block/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":false,"reason":"active_loop","message":"mock block"}}}'
EOF
$ chmod +x /tmp/ralph-iter2-block/codex1

$ PATH=/tmp/ralph-iter2-block:/usr/bin:/bin bash /Users/joel/codex1/scripts/ralph-stop-hook.sh </dev/null
ralph-stop-hook: blocking Stop - reason=active_loop
ralph-stop-hook: mock block
exit=2
```

Exit 0 on `allow: true`; exit 2 on `allow: false`. Good.

### 32. Build-gate triple

```shell
$ cargo fmt --check
# (no output; exit 0)

$ cargo clippy --all-targets -- -D warnings
   Checking codex1 v0.1.0 (/Users/joel/codex1/.claude/worktrees/agent-a8b6e479/crates/codex1)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.00s

$ cargo test --release
# (per-binary tallies summed) passed=169 failed=0 ignored=0
```

## Findings

### F8 — mission can reach `terminal_complete` with unstarted DAG tasks (P0)

- File: `crates/codex1/src/state/readiness.rs:74-82` (`tasks_complete`),
  consumer in `crates/codex1/src/cli/status/project.rs:19-20`
  (`derive_verdict` / `close_ready`).
- Claim: `tasks_complete` iterates `state.tasks.values()` only. If a
  plan-declared task is not yet in `state.tasks` (i.e. it has never
  been started), the predicate silently skips it. As soon as every
  task that has been written to `state.tasks` is terminal, verdict flips
  to `ready_for_mission_close_review` — regardless of how many planned
  tasks are still unrun.
- Prior reviewer already called this out (baseline F3 at
  `docs/audits/e2e-walkthrough-audit.md:993-998`):
  > "`tasks_complete` in `crates/codex1/src/state/readiness.rs:74-82`
  > iterates `state.tasks.values()` and, because T4 is absent, reports
  > 'all tasks terminal' while T4 itself has never been
  > terminal-marked as a task. This happens to produce the right
  > verdict (`ready_for_mission_close_review`) in the happy path, but
  > only because the predicate silently ignores missing ids."
- Iter 1's fix addressed the *symptom at review-record-clean time*
  (populate a `TaskRecord` for the review task T4 so `state.tasks` is
  a complete picture once all work has completed). It did **not**
  touch `tasks_complete`, and it did **not** pre-populate non-review
  tasks at `plan check` time. The ignore-missing-ids behaviour still
  lives.

#### Evidence 1 — divergence in the happy path at revision 6

Captured in §15 above. After the operator finishes T1 and before starting T2:

- `state.tasks = { T1: complete }` (T2, T3, T4 absent).
- `tasks_complete(state) → true` (because every entry is terminal and
  the map is non-empty).
- `derive_verdict → ReadyForMissionCloseReview`.
- `status.next_action.kind = mission_close_review`.
- Simultaneously, `task next → run_wave W2 [T2,T3]` (correctly
  derived from the plan, not `state.tasks`).

The walkthrough rubric (from the audit task) requires:
> "`task next` and `status.next_action` always agree on kind + ids at
> every step."

They do not. This disagreement alone is a contract violation. The
P0 severity below is because the disagreement is exploitable.

#### Evidence 2 — standalone reproduction: close mission with 3 of 4 tasks unrun

Ran an independent mission (`/tmp/codex1-iter2-bug-repro/`) with the
same 4-task PLAN.yaml. After finishing only T1, called
`close record-review --clean` and `close complete`; both succeeded.

```shell
$ cd /tmp/codex1-iter2-bug-repro
# (same init / ratify / choose-level / scaffold / plan check as happy path;
#  plan is the same 4-task DAG.)

$ codex1 --json task start T1 --mission demo     # revision 4 → 5
$ codex1 --json task finish T1 --mission demo --proof specs/T1/PROOF.md
# revision 6, T1 complete, T2/T3/T4 never started.

$ codex1 --json close check --mission demo
{
  "ok": true, "revision": 6,
  "data": {
    "blockers": [
      { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" }
    ],
    "ready": false,
    "verdict": "ready_for_mission_close_review"    # <-- LIE: 3 of 4 tasks unrun
  }
}

$ codex1 --json close record-review --clean --reviewers test --mission demo
{
  "ok": true, "revision": 7,
  "data": {
    "consecutive_dirty": 0, "dry_run": false, "findings_file": null,
    "replan_triggered": false, "review_state": "passed",
    "reviewers": ["test"], "verdict": "clean"
  }
}

$ codex1 --json close check --mission demo
{ "ok": true, "revision": 7, "data": { "blockers": [], "ready": true, "verdict": "mission_close_review_passed" } }

$ codex1 --json close complete --mission demo
{
  "ok": true, "revision": 8,
  "data": {
    "closeout_path": "/private/tmp/codex1-iter2-bug-repro/PLANS/demo/CLOSEOUT.md",
    "dry_run": false, "mission_id": "demo",
    "terminal_at": "2026-04-20T18:07:48.427971Z"
  }
}
```

Resulting `CLOSEOUT.md`:

```markdown
# CLOSEOUT — demo
**Terminal at:** 2026-04-20T18:07:48.427971Z
**Final revision:** 8
**Planning level:** medium

## Outcome
…

## Tasks

| ID | Status | Proof |
|---|---|---|
| T1 | complete | specs/T1/PROOF.md |

## Reviews

| Review ID | Verdict | Reviewers | Findings |
|---|---|---|---|
| MC | clean | — | — |

## Mission-close review
Clean on the first round.
```

The mission terminated with `verdict: terminal_complete` and a
CLOSEOUT.md that silently drops T2, T3, and T4. No warning, no
refusal, no error.

#### Evidence 3 — ready_tasks / phase disagreement at terminal

After the bug-repro `close complete`:

```shell
$ codex1 --json status --mission demo
{
  "ok": true, "revision": 8,
  "data": {
    "close_ready": false,
    "next_action": { "hint": "Mission is terminal.", "kind": "closed" },
    "phase": "terminal",
    "ready_tasks": ["T2","T3"],      # <-- but mission is terminal!
    "review_required": [],
    "verdict": "terminal_complete",
    …
  }
}
```

`phase: terminal` alongside `ready_tasks: [T2, T3]` is an internally
inconsistent envelope. Downstream readers of `ready_tasks` will see a
contradiction with `phase`.

#### Severity

P0. The happy-path walkthrough (§§4-29) does reach a correct
`terminal_complete` with all tasks present, but only because the
operator happens to run every task in order before calling
`close record-review`. Any driver (skill, Ralph harness, human) that
trusts `status.next_action` over `task next` after any single finish
will incorrectly be told "mission-close review" and, by following the
documented close flow, silently terminate a DAG with unrun tasks. The
`close record-review` gate is ineffective.

#### Suggested fix location (non-prescriptive)

Either:

1. Pre-populate `state.tasks` at `plan check` time with a Pending
   entry per declared task. `tasks_complete` would then naturally
   reject because not every entry is terminal.
2. Change `tasks_complete` in `crates/codex1/src/state/readiness.rs:74-82`
   to iterate the plan-enumerated IDs (via the same PLAN.yaml parse
   that `status/next_action.rs` already does) rather than
   `state.tasks.values()`, and require each plan ID to have a
   terminal-state `TaskRecord`.

Either approach closes the gate end-to-end.

## Clean checks

- [x] `codex1 --help` lists every documented command (11 subcommands:
  init, doctor, hook, outcome, plan, task, review, replan, loop,
  close, status).
- [x] `codex1 --json doctor` returns `ok: true` without auth
  (`required: false`). See §3.
- [x] `plan choose-level` before `outcome ratify` fails with
  `OUTCOME_NOT_RATIFIED` (iter 1 CLI-contract P1-1 fix). See §5.
- [x] Every CLI call in the happy path returns a well-formed JSON
  envelope with the contract keys (`ok`, `mission_id`, `revision`,
  `data` on success; `ok`, `code`, `message`, `retryable` on error).
- [x] **Baseline F1 fixed**: at revision 10 (T2/T3 in AwaitingReview),
  `task next` and `status.next_action` both report
  `run_review T4 targets=[T2,T3]`, and `status.ready_tasks == [T4]`
  (not [T2,T3]). See §18. Non-review AwaitingReview tasks are
  correctly excluded from `ready_tasks`.
- [x] **Baseline F2 fixed**: at revisions 12 and 13 (T4 clean
  recorded), `status.review_required` is `[]` (not the stale
  `[{T4, [T2,T3]}]`). See §22 and §25. `state.reviews` is now
  consulted.
- [x] **Baseline F3 (surface-symptom) fixed**: after
  `review record T4 --clean`, `task status T4 → status: complete`
  with `finished_at` set. T4 is present in `state.tasks` and in the
  CLOSEOUT.md Tasks table. See §21 and §29.
- [x] **Baseline F4 fixed**: `review packet T4` emits a `proofs` field
  (no `target_proofs`). See §20.
- [x] **Baseline F5 still additive (expected)**: `review record`
  envelope still carries `findings_file`, `replan_triggered`,
  `warnings`; per iter 1 docs these are documented as additive
  extras.
- [x] **Baseline F6 documentation on record**: relative
  `specs/T1/PROOF.md` (mission-dir-relative) works without surprise.
  See §14.
- [x] **Baseline F7 fixed**: CLOSEOUT.md Tasks table includes
  T1, T2, T3 **and T4** (review kind). See §29.
- [x] `status.close_ready` and `close check.ready` agree at every
  captured phase (§23: both false; §25: both true; §27 terminal:
  both false, per contract `close_ready == (verdict ==
  mission_close_review_passed)`).
- [x] Mission reaches `verdict: terminal_complete`. See §27.
- [x] Second `close complete` returns `TERMINAL_ALREADY_COMPLETE`
  with `retryable: false`. See §28.
- [x] EVENTS.jsonl is append-only, `seq` monotonically increasing,
  timestamps chronological. 14 events. See §30.
- [x] Ralph hook exits 0 on `stop.allow: true` and exits 2 on
  `stop.allow: false` (with reason / message on stderr). See §31.
- [x] `cargo fmt --check` clean. See §32.
- [x] `cargo clippy --all-targets -- -D warnings` clean. See §32.
- [x] `cargo test --release` passes (169 / 169). See §32.

One finding: **F8 (P0)** — mission-close gate is ineffective against
unstarted tasks because `tasks_complete` iterates `state.tasks.values()`
instead of the plan-enumerated task IDs. Same root cause the baseline
reviewer flagged under F3; iter 1's fix addressed the review-task
symptom but not the predicate. Reproduction shows a mission can reach
`terminal_complete` with 3 of 4 DAG tasks never started.
