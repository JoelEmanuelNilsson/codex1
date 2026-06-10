# Iter 4 end-to-end walkthrough audit

Branch: `audit-iter4-e2e-post5a16894` (requested name `audit-iter4-e2e`
was locked in another worktree at 2ef3ce7; renamed to keep the audit
rooted at 5a16894 as requested)
Commit: 5a16894 (`iter3-wave-fix: close/check blocker enum now walks plan.task_ids`)
Audited on: 2026-04-21 (UTC)
Build: `make install-local` (release, 17.08s) — PASS
`cargo fmt --check` — PASS
`cargo clippy --all-targets -- -D warnings` — PASS
`cargo test --release` — **169 passed / 0 failed** across 19 test binaries (see note).

> Note on test count: commits 958d2f1, 2ef3ce7 and 5a16894 all say
> "170 passing" in their commit messages. The actual count at each of
> those commits, and at HEAD, is 169. 958d2f1's "169 + one new F8
> regression test" was in fact a new **fixture** inside the existing
> `#[test] fn status_agrees_with_readiness_helpers_for_all_fixtures` —
> adding a fixture does not add a test function. Off-by-one in the
> commit arithmetic, no runtime regression; left intentionally out of
> the P-findings list.

## Summary

**FAIL** — the walkthrough reaches `terminal_complete` end to end, CLOSEOUT
cites all four task IDs, status and close-check agree on `close_ready` at
every phase, and the Ralph stop hook exits 0/2 per the contract. The F3
wave fix (5a16894) works as advertised. The F8 fix (958d2f1) works as
advertised. The baseline F1..F7 fixes in 6473650 all verify empirically.

**F11 does not**. Commit 2ef3ce7 describes tightening the `plan check`
short-circuit to require `!task_ids_missing`, but the actual diff only
contains the F9 clippy fix (`s.plan.task_ids.clone_from(&task_ids_to_record)`).
The upgrade trap is still live: missions locked by a pre-F8 binary
cannot recover, and `plan check` — the documented recovery path — does
not backfill `plan.task_ids` (see §F11 below, with a reproducible
sequence bricking a fresh mission at `continue_required` with
`next_action: blocked`).

Findings: **0 P0 · 1 P1 · 0 P2**.

## Baseline + iter fix verification

| Finding | Severity (orig) | Fix commit | Verified here | Result |
|---|---|---|---|---|
| F1 — `status.next_action` vs `task next` post-AwaitingReview | P1 | 6473650 | §W22 (rev 10: both say `run_review T4`) | FIXED |
| F2 — `status.review_required` stale after clean | P1 | 6473650 | §W26 (rev 12: `review_required: []`) | FIXED |
| F3 (baseline) — clean review creates TaskRecord for review task | P1 | 6473650 | §W29 (`state.tasks.T4.status = complete`) | FIXED |
| F4 — packet field `target_proofs` → `proofs` | P1 | 6473650 | §W24 (`proofs: [...]`) | FIXED |
| F5 — record envelope fields documented | P2 | 6473650 | §W25 (contract table reflects extras) | FIXED |
| F6 — proof path resolution documented | P2 | 6473650 | contract update in `docs/cli-contract-schemas.md` | FIXED (doc only) |
| F7 — CLOSEOUT Tasks table omits T4 | P2 | 6473650 | §W32 (Tasks row "T4 · complete · —") | FIXED |
| F8 — `tasks_complete` consulted wrong source | P0 | 958d2f1 | §F8 below | FIXED |
| F9 — clippy `assigning_clones` in `plan/check.rs` | P1 | 2ef3ce7 | `cargo clippy` clean | FIXED |
| F10 — two status tests failed after F8 landed | P1 | 2ef3ce7 | `cargo test --release` clean | FIXED |
| F11 — plan-check upgrade trap (task_ids missing) | P2 | 2ef3ce7 (**claimed, not in source**) | §F11 below | **NOT FIXED** |
| F3 (wave) — `close check` blocker loop walked `state.tasks` | P1 | 5a16894 | §F3 below | FIXED |

## F3 (wave) regression check — partial-completion `close check`

Run directly against the walkthrough mission at revision 6 (T1 complete,
T2/T3/T4 never started, `plan.task_ids = [T1,T2,T3,T4]`):

```shell
$ cd /tmp/codex1-iter4-demo && codex1 --json close check --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 6,
  "data": {
    "blockers": [
      { "code": "TASK_NOT_READY", "detail": "T2 has not started" },
      { "code": "TASK_NOT_READY", "detail": "T3 has not started" },
      { "code": "TASK_NOT_READY", "detail": "T4 has not started" }
    ],
    "ready": false,
    "verdict": "continue_required"
  }
}
```

Exactly the three DAG-node-missing blockers promised by the fix. The
5a16894 commit delivers what it advertises. Matching `status` at the same
revision returns `verdict: continue_required` and `close_ready: false`
(§W18 in the walkthrough).

## F8 regression check — partial-completion `status`

Same state as above:

```shell
$ codex1 --json status --mission demo
{ ... "verdict": "continue_required", "close_ready": false,
  "next_action": { "kind": "run_wave", "tasks": ["T2","T3"], "wave_id": "W2" },
  "ready_tasks": ["T2","T3"], ... }
```

Verdict is `continue_required`, not `ready_for_mission_close_review`.
F8 FIXED.

## F11 upgrade-trap check — **FAIL**

Reproducible sequence in `/tmp/codex1-iter4-f11`. Mission init, ratify,
choose-level, scaffold, and (most importantly) the first `plan check`
write `state.plan.task_ids = [T1,T2,T3,T4]` at revision 4. Then
simulate a pre-F8 binary having written STATE.json by dropping the
`task_ids` field:

```python
s = json.load(open('PLANS/f11/STATE.json'))
s['plan'].pop('task_ids', None)            # simulate pre-F8 binary
json.dump(s, open('PLANS/f11/STATE.json','w'), indent=2)
```

Re-run `codex1 plan check --mission f11` — the documented recovery path.
Expected (per 2ef3ce7 commit message): backfill `task_ids` and bump
revision to 5, then remain idempotent at revision 5 on a second re-run.

Actual:

```shell
$ codex1 --json plan check --mission f11
{ "ok": true, "revision": 4, "data": { "locked": true, ... } }
$ cat PLANS/f11/STATE.json | jq '.revision, .plan.task_ids'
4
null                  # task_ids still absent, no backfill

$ codex1 --json plan check --mission f11       # second re-run
{ "ok": true, "revision": 4, "data": { "locked": true, ... } }   # same
```

Revision stays at 4 across both re-runs and `task_ids` is never
restored — the short-circuit at `cli/plan/check.rs:63-64` fires on hash
match alone and never enters the mutation closure.

End-to-end consequence (mission bricked): with all four tasks forced
into `state.tasks` as Complete (simulating an older binary that finished
them without populating `plan.task_ids`), `status` returns:

```json
{
  "verdict": "continue_required",
  "close_ready": false,
  "next_action": { "kind": "blocked",
                   "reason": "No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json." },
  "ready_tasks": []
}
```

`tasks_complete` (`state/readiness.rs:80-93`) short-circuits to `false`
when `state.plan.task_ids.is_empty()`, so `derive_verdict` can never
reach `MissionCloseReviewPassed`. `plan check` is the only user-visible
route that would ever write `task_ids`, and the short-circuit blocks
that write on any locked plan whose PLAN.yaml hash is unchanged. The
mission cannot reach terminal without a hand-edit.

This is worse than the original F11 (which was P2). The claim that a
fix landed in 2ef3ce7 is demonstrably incorrect. See §Finding F11
below.

## Walkthrough transcript (condensed)

All envelopes captured against `/tmp/codex1-iter4-demo`, mission `demo`.

### W1. Install

```shell
$ cd /Users/joel/codex1/.claude/worktrees/agent-a4aa8025 && time make install-local
   <snip: 60+ release compile lines>
    Finished `release` profile [optimized] target(s) in 17.08s
cp target/release/codex1 /Users/joel/.local/bin/codex1
Installed codex1 to /Users/joel/.local/bin/codex1
make install-local  39.95s user 2.87s system 250% cpu 17.127 total
```

### W2. Verify from /tmp

```shell
$ cd /tmp && command -v codex1
/Users/joel/.local/bin/codex1
$ codex1 --version
codex1 0.1.0
$ codex1 --help      # 11 subcommands: init, doctor, hook, outcome,
                     # plan, task, review, replan, loop, close, status
```

### W3. Doctor (from /tmp)

```shell
$ cd /tmp && codex1 --json doctor
{ "ok": true, "data": {
    "auth": { "required": false, ... },
    "config": { "exists": false, "path": "/Users/joel/.codex1/config.toml" },
    "cwd": "/private/tmp",
    "install": { "codex1_on_path": "/Users/joel/.local/bin/codex1", "home_local_bin_writable": true, ... },
    "version": "0.1.0", "warnings": [] } }
```

### W4. Init

```shell
$ cd /tmp/codex1-iter4-demo && codex1 --json init --mission demo
{ "ok": true, "mission_id": "demo", "revision": 0,
  "data": { "created": { "events": ".../EVENTS.jsonl",
                         "mission_dir": ".../PLANS/demo",
                         "outcome": ".../OUTCOME.md", "plan": ".../PLAN.yaml",
                         "reviews_dir": ".../reviews", "specs_dir": ".../specs",
                         "state": ".../STATE.json" },
            "next_action": { "command": "$clarify", "kind": "clarify", ... } } }
```

### W5. Status after init

```shell
$ codex1 --json status --mission demo
{ ... "revision": 0, "data": { "phase": "clarify", "outcome_ratified": false,
      "plan_locked": false, "verdict": "needs_user", "close_ready": false,
      "next_action": { "kind": "clarify" },
      "stop": { "allow": true, "reason": "idle", ... } } }
```

### W6. Outcome check (after OUTCOME.md filled)

```shell
$ codex1 --json outcome check --mission demo
{ "ok": true, "revision": 0,
  "data": { "missing_fields": [], "placeholders": [], "ratifiable": true } }
```

### W7. Outcome ratify

```shell
$ codex1 --json outcome ratify --mission demo
{ "ok": true, "revision": 1,
  "data": { "mission_id": "demo", "phase": "plan",
            "ratified_at": "2026-04-21T07:43:48.908881Z" } }
```

### W8. Status after ratify

```shell
$ codex1 --json status --mission demo
{ ... "revision": 1, "phase": "plan", "outcome_ratified": true,
      "verdict": "needs_user", "next_action": { "command": "$plan",
      "kind": "plan", "hint": "Draft and lock PLAN.yaml." } }
```

### W9. Plan choose-level

```shell
$ codex1 --json plan choose-level --mission demo --level medium
{ "ok": true, "revision": 2,
  "data": { "effective_level": "medium", "requested_level": "medium",
            "next_action": { "args": ["codex1","plan","scaffold","--level","medium"],
                             "kind": "plan_scaffold" } } }
```

### W10. Plan scaffold

```shell
$ codex1 --json plan scaffold --mission demo --level medium
{ "ok": true, "revision": 3,
  "data": { "level": "medium", "specs_created": [], "wrote": "PLANS/demo/PLAN.yaml" } }
```

PLAN.yaml was then filled in with a 4-task DAG (T1 root → T2/T3 fan-out
→ T4 review targeting T2+T3) and SPEC.md files were written under
`PLANS/demo/specs/T{1..4}/`.

### W11. Plan check

```shell
$ codex1 --json plan check --mission demo
{ "ok": true, "revision": 4,
  "data": { "hard_evidence": 0, "locked": true,
            "plan_hash": "sha256:5463a4c76b...", "review_tasks": 1, "tasks": 4 } }
```

### W12. Plan waves

```shell
$ codex1 --json plan waves --mission demo
{ "ok": true, "revision": 4,
  "data": { "all_tasks_complete": false, "current_ready_wave": "W1",
            "waves": [
              { "wave_id": "W1", "tasks": ["T1"], "parallel_safe": true, "blockers": [] },
              { "wave_id": "W2", "tasks": ["T2","T3"], "parallel_safe": true, "blockers": [] },
              { "wave_id": "W3", "tasks": ["T4"], "parallel_safe": true, "blockers": [] } ] } }
```

### W13. Plan graph (mermaid)

```shell
$ codex1 --json plan graph --mission demo --format mermaid
{ "ok": true, "revision": 4,
  "data": { "mermaid": "flowchart TD\n    classDef complete ...\n    T1 --> T2\n    T1 --> T3\n    T2 --> T4\n    T3 --> T4\n    class T1 ready\n    class T2 blocked\n    class T3 blocked\n    class T4 blocked\n" } }
```

### W14. Task next (fresh plan)

```shell
$ codex1 --json task next --mission demo
{ "ok": true, "revision": 4, "data": { "next": { "kind": "run_task", "task_id": "T1", "task_kind": "code" } } }
```

### W15. Task start T1

```shell
$ codex1 --json task start T1 --mission demo
{ "ok": true, "revision": 5,
  "data": { "idempotent": false, "started_at": "2026-04-21T07:46:37.05579Z",
            "status": "in_progress", "task_id": "T1" } }
```

### W16. Task finish T1

```shell
$ codex1 --json task finish T1 --mission demo --proof specs/T1/PROOF.md
{ "ok": true, "revision": 6,
  "data": { "finished_at": "2026-04-21T07:46:45.831854Z",
            "proof_path": "specs/T1/PROOF.md",
            "status": "complete", "task_id": "T1" } }
```

(T1 has no `review_target`, so it transitions directly to `complete`.)

### W17. F3+F8 regression sampling — `/tmp/codex1-iter4-state-after-T1.json`

State snapshotted at revision 6 (T1 complete, T2/T3/T4 never started).
See §F3 and §F8 sections above for full envelopes; both pass.

### W18. Task next after T1

```shell
$ codex1 --json task next --mission demo
{ "ok": true, "revision": 6,
  "data": { "next": { "kind": "run_wave", "parallel_safe": true,
                      "tasks": ["T2","T3"], "wave_id": "W2" } } }
```

### W19. Task start T2

```shell
$ codex1 --json task start T2 --mission demo
{ "ok": true, "revision": 7,
  "data": { "idempotent": false, "started_at": "2026-04-21T07:50:53.112571Z",
            "status": "in_progress", "task_id": "T2" } }
```

### W20. Task finish T2

```shell
$ codex1 --json task finish T2 --mission demo --proof specs/T2/PROOF.md
{ "ok": true, "revision": 8,
  "data": { "finished_at": "2026-04-21T07:51:01.174843Z",
            "proof_path": "specs/T2/PROOF.md",
            "status": "awaiting_review", "task_id": "T2" } }
```

### W21. Task start T3

```shell
$ codex1 --json task start T3 --mission demo
{ "ok": true, "revision": 9,
  "data": { "idempotent": false, "started_at": "2026-04-21T07:51:04.174747Z",
            "status": "in_progress", "task_id": "T3" } }
```

### W22. Task finish T3

```shell
$ codex1 --json task finish T3 --mission demo --proof specs/T3/PROOF.md
{ "ok": true, "revision": 10,
  "data": { "finished_at": "2026-04-21T07:51:11.038284Z",
            "proof_path": "specs/T3/PROOF.md",
            "status": "awaiting_review", "task_id": "T3" } }
```

### W23. F1 verification — status vs `task next` post-AwaitingReview

```shell
$ codex1 --json task next --mission demo
{ "ok": true, "revision": 10,
  "data": { "next": { "kind": "run_review", "targets": ["T2","T3"], "task_id": "T4" } } }

$ codex1 --json status --mission demo
{ ... "revision": 10, "verdict": "continue_required", "close_ready": false,
       "ready_tasks": ["T4"],
       "review_required": [ { "targets": ["T2","T3"], "task_id": "T4" } ],
       "next_action": { "command": "$review-loop", "kind": "run_review",
                        "review_task_id": "T4", "targets": ["T2","T3"] } }
```

Both surfaces agree on `run_review T4`. `ready_tasks` is `[T4]`, not
`[T2,T3]` — F1 FIXED.

### W24. Review start T4

```shell
$ codex1 --json review start T4 --mission demo
{ "ok": true, "revision": 11,
  "data": { "boundary_revision": 11, "review_task_id": "T4",
            "targets": ["T2","T3"], "verdict": "pending" } }
```

### W25. Review packet T4 (F4 verification)

```shell
$ codex1 --json review packet T4 --mission demo
{ "ok": true, "revision": 11,
  "data": {
    "task_id": "T4", "targets": ["T2","T3"], "review_profile": "code_bug_correctness",
    "diffs": [ { "path": "src/T2/**" }, { "path": "src/T3/**" } ],
    "proofs": [ "PLANS/demo/specs/T2/PROOF.md", "PLANS/demo/specs/T3/PROOF.md" ],
    "mission_id": "demo", "mission_summary": "|\nA complete mission trajectory ...",
    "profiles": [], "reviewer_instructions": "You are a Codex1 reviewer. Do not edit files. ...",
    "target_specs": [
      { "task_id": "T2", "spec_path": "specs/T2/SPEC.md", "spec_excerpt": "# T2 — Parallel branch A ..." },
      { "task_id": "T3", "spec_path": "specs/T3/SPEC.md", "spec_excerpt": "# T3 — Parallel branch B ..." } ] } }
```

`proofs` uses the canonical contract name. F4 FIXED.

### W26. Review record T4 --clean (F5)

```shell
$ codex1 --json review record T4 --clean --reviewers e2e-auditor --mission demo
{ "ok": true, "revision": 12,
  "data": { "category": "accepted_current", "findings_file": null,
            "replan_triggered": false, "review_task_id": "T4",
            "reviewers": ["e2e-auditor"], "verdict": "clean",
            "warnings": [] } }
```

`findings_file`, `replan_triggered`, `warnings` are documented additive
fields (contract §review record). F5 FIXED.

### W27. F2 + baseline-F3 verification

```shell
$ codex1 --json status --mission demo
{ ... "revision": 12, "verdict": "ready_for_mission_close_review",
       "close_ready": false, "ready_tasks": [],
       "review_required": [],               # F2: empty, not stale with T4
       "next_action": { "command": "$review-loop (mission-close)",
                        "kind": "mission_close_review",
                        "hint": "All tasks complete; run the mission-close review." } }

$ jq '.tasks.T4.status' PLANS/demo/STATE.json
"complete"                                  # Baseline F3: TaskRecord exists
```

F2 and baseline-F3 FIXED.

### W28. Task next after T4 clean

```shell
$ codex1 --json task next --mission demo
{ "ok": true, "revision": 12,
  "data": { "next": { "kind": "mission_close_review",
                      "reason": "all tasks complete or superseded" } } }
```

(Previously in baseline this still returned `run_task T4` — FIXED.)

### W29. Close check before mission-close review

```shell
$ codex1 --json close check --mission demo
{ "ok": true, "revision": 12,
  "data": { "blockers": [ { "code": "CLOSE_NOT_READY",
                            "detail": "mission-close review has not started" } ],
            "ready": false, "verdict": "ready_for_mission_close_review" } }
```

`status` and `close check` agree.

### W30. Close record-review --clean

```shell
$ codex1 --json close record-review --clean --reviewers mission-close-auditor --mission demo
{ "ok": true, "revision": 13,
  "data": { "consecutive_dirty": 0, "dry_run": false, "findings_file": null,
            "replan_triggered": false, "review_state": "passed",
            "reviewers": ["mission-close-auditor"], "verdict": "clean" } }
```

### W31. Status after mission-close review

```shell
$ codex1 --json status --mission demo
{ ... "revision": 13, "verdict": "mission_close_review_passed",
       "close_ready": true,
       "next_action": { "command": "codex1 close complete",
                        "kind": "close",
                        "hint": "Mission-close review passed; finalize close." } }

$ codex1 --json close check --mission demo
{ ... "revision": 13, "data": { "blockers": [], "ready": true,
                                "verdict": "mission_close_review_passed" } }
```

Both surfaces agree on `close_ready: true`.

### W32. Loop activate / pause / resume / deactivate

```shell
$ codex1 --json loop activate --mode execute --mission demo
{ "ok": true, "revision": 14, "data": { "active": true, "mode": "execute",
                                        "paused": false, ... } }
# status: stop = { allow: true, reason: idle,
#                  message: "Loop is active but verdict allows stop." }

$ codex1 --json loop pause --mission demo
{ "ok": true, "revision": 15, "data": { "active": true, "paused": true, ... } }
# status: stop = { allow: true, reason: paused, message: "Loop is paused; ..." }

$ codex1 --json loop resume --mission demo
{ "ok": true, "revision": 16, "data": { "active": true, "paused": false, ... } }

$ codex1 --json loop deactivate --mission demo
{ "ok": true, "revision": 17, "data": { "active": false, "mode": "none", ... } }
```

### W33. Close complete (terminal)

```shell
$ codex1 --json close complete --mission demo
{ "ok": true, "revision": 18,
  "data": { "closeout_path": ".../CLOSEOUT.md", "dry_run": false,
            "mission_id": "demo", "terminal_at": "2026-04-21T07:52:12.821279Z" } }
```

### W34. Terminal status

```shell
$ codex1 --json status --mission demo
{ ... "revision": 18, "phase": "terminal", "verdict": "terminal_complete",
       "close_ready": false,
       "stop": { "allow": true, "reason": "terminal", ... },
       "next_action": { "kind": "closed", "hint": "Mission is terminal." } }
```

### W35. Close complete idempotency

```shell
$ codex1 --json close complete --mission demo
{ "ok": false, "code": "TERMINAL_ALREADY_COMPLETE",
  "message": "Mission is already terminal (closed at 2026-04-21T07:52:12.821279Z)",
  "hint": "Start a new mission; a terminal mission cannot be reopened.",
  "retryable": false,
  "context": { "closed_at": "2026-04-21T07:52:12.821279Z" } }
```

Contract-documented error code. Good.

### W36. CLOSEOUT.md

```markdown
# CLOSEOUT — demo

**Terminal at:** 2026-04-21T07:52:12.821279Z
**Final revision:** 18
**Planning level:** medium

## Outcome
A complete mission trajectory drives from `codex1 init` through
`codex1 close complete` ...

## Tasks
| ID | Status   | Proof               |
|----|----------|---------------------|
| T1 | complete | specs/T1/PROOF.md   |
| T2 | complete | specs/T2/PROOF.md   |
| T3 | complete | specs/T3/PROOF.md   |
| T4 | complete | —                   |

## Reviews
| Review ID | Verdict | Reviewers              | Findings |
|-----------|---------|------------------------|----------|
| T4        | clean   | e2e-auditor            | —        |
| MC        | clean   | —                      | —        |

## Mission-close review
Clean on the first round.
```

All four task IDs (T1, T2, T3, T4) appear in the Tasks table. F7 FIXED.

### W37. EVENTS.jsonl — append-only, monotonic

```text
{"seq":1,"at":"2026-04-21T07:43:48.921418Z","kind":"outcome.ratified",...}
{"seq":2,"at":"2026-04-21T07:43:56.249337Z","kind":"plan.choose_level",...}
{"seq":3,"at":"2026-04-21T07:44:01.836949Z","kind":"plan.scaffold",...}
{"seq":4,                              "kind":"plan.checked",...}
{"seq":5,                              "kind":"task.started",...}          # T1
{"seq":6,                              "kind":"task.finished",...}         # T1
{"seq":7,                              "kind":"task.started",...}          # T2
{"seq":8,                              "kind":"task.finished",...}         # T2
{"seq":9,                              "kind":"task.started",...}          # T3
{"seq":10,                             "kind":"task.finished",...}         # T3
{"seq":11,                             "kind":"review.started",...}        # T4
{"seq":12,                             "kind":"review.recorded.clean",...} # T4
{"seq":13,                             "kind":"close.review.clean",...}
{"seq":14,                             "kind":"loop.activated",...}
{"seq":15,                             "kind":"loop.paused",...}
{"seq":16,                             "kind":"loop.resumed",...}
{"seq":17,                             "kind":"loop.deactivated",...}
{"seq":18,"at":"2026-04-21T07:52:12.828307Z","kind":"close.complete",...}
```

18 events; `seq` strictly increasing; timestamps chronological.

### W38. Ralph hook manual test

Two mock `codex1` binaries placed on a scratch PATH and the Stop hook
driven against each:

```shell
$ cat > /tmp/ralph-test-pass/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":true,"reason":"idle","message":"mock idle"}}}'
EOF
$ chmod +x /tmp/ralph-test-pass/codex1

$ PATH=/tmp/ralph-test-pass:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
exit=0

$ cat > /tmp/ralph-test-block/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":false,"reason":"active_loop","message":"mock block"}}}'
EOF
$ chmod +x /tmp/ralph-test-block/codex1

$ PATH=/tmp/ralph-test-block:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
ralph-stop-hook: blocking Stop - reason=active_loop
ralph-stop-hook: mock block
exit=2
```

Exits 0 when `stop.allow: true`, 2 when `stop.allow: false`. Contract
honoured.

## stop.allow table (captured at each phase)

| Phase / revision | loop.active | loop.paused | verdict | stop.allow | stop.reason |
|---|---|---|---|---|---|
| after init (rev 0) | false | false | needs_user | true | idle |
| after ratify (rev 1) | false | false | needs_user | true | idle |
| after T1 finish (rev 6) | false | false | continue_required | true | idle |
| after T3 finish (rev 10) | false | false | continue_required | true | idle |
| after T4 clean (rev 12) | false | false | ready_for_mission_close_review | true | idle |
| after mission-close review (rev 13) | false | false | mission_close_review_passed | true | idle |
| loop activated (rev 14) | true | false | mission_close_review_passed | true | idle |
| loop paused (rev 15) | true | true | mission_close_review_passed | true | paused |
| terminal (rev 18) | false | false | terminal_complete | true | terminal |

## Findings

### F11 (iter4) — `plan check` does not backfill `plan.task_ids` on upgrade (P1)

- File: `crates/codex1/src/cli/plan/check.rs:61-84`
- Also `crates/codex1/src/state/readiness.rs:80-93` (consumer that
  short-circuits to `false` when `task_ids` is empty).
- Commit 2ef3ce7 message: *"Tightened the short-circuit to require
  `!task_ids_missing` in addition to `hash_matches`, so an upgraded
  binary re-runs the mutation exactly once to backfill and then returns
  to the idempotent path."* — **not present in the 2ef3ce7 diff.** The
  diff touched only one line:
  `s.plan.task_ids = task_ids_to_record.clone();` →
  `s.plan.task_ids.clone_from(&task_ids_to_record);` (the F9 clippy
  fix). No change to the short-circuit condition.
- Current short-circuit (unchanged since the F8 fix):
  ```rust
  let already_locked_same =
      current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());
  if ctx.dry_run || already_locked_same { /* return without mutation */ }
  ```
- Empirical reproduction (fresh mission in `/tmp/codex1-iter4-f11`):
  1. init → ratify → choose-level → scaffold → plan check:
     `revision=4`, `plan.task_ids = [T1,T2,T3,T4]`.
  2. Drop `plan.task_ids` from STATE.json (simulate pre-F8 binary).
  3. `plan check` (re-run 1): `revision=4` (no bump); `task_ids` still
     absent after the call.
  4. `plan check` (re-run 2): `revision=4` again; still no backfill.
- Terminal consequence: seed all four tasks as Complete in
  `state.tasks` with `task_ids` still absent. `readiness::tasks_complete`
  short-circuits to `false` (line 82-86: "Plan isn't locked (or the
  plan has zero tasks) … treat as 'not done'"). `status` returns
  `verdict: continue_required`, `next_action: { kind: blocked, reason:
  "No ready wave derivable …" }`, `ready_tasks: []`. The mission is
  bricked: there is no user-visible command that can repair
  `plan.task_ids` short of hand-editing STATE.json.
- Severity: **P1.** The original F11 was P2 (upgrade trap affecting
  in-the-wild missions). Upgrading P2 to P1 here because the claimed
  fix is not in source — the commit history overstates what was
  repaired, which is itself a credibility problem for the next audit
  iteration. Severity is *not* P0 because a fresh post-5a16894 mission
  writes `task_ids` correctly; the trap requires a STATE.json that was
  produced by an older binary.
- Recommended patch (for iter5, not applied here per the audit's
  "do NOT modify source" constraint):
  ```rust
  let already_locked_same =
      current.plan.locked
          && current.plan.hash.as_deref() == Some(hash.as_str())
          && !current.plan.task_ids.is_empty();
  ```
  plus a unit test in `tests/plan_check.rs` that seeds an already-locked
  state with empty `task_ids`, runs `plan check`, asserts
  `state.plan.task_ids` is non-empty and revision bumped by exactly 1,
  then runs `plan check` again and asserts revision unchanged.

## Clean checks

- [x] `codex1 --help` lists every documented command (11 subcommands:
  init, doctor, hook, outcome, plan, task, review, replan, loop,
  close, status — matches the contract-referenced surface).
- [x] `codex1 --json doctor` returns `ok: true` without auth
  (`required: false`).
- [x] Full mission reaches `terminal_complete` (status at revision 18).
- [x] `CLOSEOUT.md` is written and cites each task ID (T1, T2, T3, T4
  all present in the Tasks table).
- [x] `EVENTS.jsonl` is append-only and monotonic (seq 1..18;
  timestamps chronological).
- [x] `codex1 status` and `codex1 close check` agree on `close_ready`
  at every captured phase (see §W29, §W31, §F3, §F8).
- [x] Ralph hook shell exits 0 when `stop.allow: true`, 2 when false
  (§W38).
- [x] `cargo fmt --check` clean.
- [x] `cargo clippy --all-targets -- -D warnings` clean.
- [x] `cargo test --release` all 169 tests pass.

All clean checks pass. One P1 finding (F11 iter4) stands: the claimed
upgrade-trap fix is not in source, and the trap is empirically
reproducible from a freshly locked plan.
