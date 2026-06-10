# Iter 5 end-to-end walkthrough audit

Branch: `audit-iter5-e2e-post-c5e07ad`
Commit: `c5e07ad` (`iter4-fix-followup: regression test for F11 upgrade trap`)
Audited on: 2026-04-21 (UTC)

Build / install: `make install-local` (release, 22.94s) — PASS
`cargo fmt --check` — PASS
`cargo clippy --all-targets -- -D warnings` — PASS
`cargo test --release` — **170 passed / 0 failed** across 19 test binaries (17
integration binaries with ≥1 test + 2 unittest binaries, one of which has
zero tests).

> Test count reconciliation: iter4 measured 169 and called the earlier
> commits' "170 passing" claims off-by-one. c5e07ad added exactly one new
> test function (`plan_check_backfills_missing_task_ids_and_then_stays_idempotent`
> in `tests/plan_check.rs`), bringing the real count to 170. That
> matches the harness expectation and the per-binary tally below:
>
> | Binary | Tests |
> |---|---|
> | unittests (src/lib.rs) | 10 |
> | unittests (src/bin/codex1.rs) | 0 |
> | close | 17 |
> | e2e_full_mission | 1 |
> | e2e_ralph_contract | 5 |
> | e2e_replan_trigger | 1 |
> | foundation | 10 |
> | loop_ | 20 |
> | outcome | 8 |
> | plan_check | 13 |
> | plan_scaffold | 10 |
> | plan_waves | 8 |
> | ralph_hook | 6 |
> | replan | 10 |
> | review | 16 |
> | status | 14 |
> | status_close_agreement | 3 |
> | task | 18 |
> | **Total** | **170** |

## Summary

**PASS** — the full walkthrough reaches `terminal_complete` end to end at
revision 18, CLOSEOUT.md cites every task id, `status` and `close check`
agree on `close_ready` at every captured phase, the Ralph stop hook
exits 0/2 per contract, every CLI surface returns well-formed JSON, and
the three regression guards (F3-wave, F8, F11) all hold empirically.

**The F11 upgrade-trap fix is now live and verified.** Commit b212ca8
restored the short-circuit guard in `cli/plan/check.rs:69-72` and
commit c5e07ad added a regression test. Reproducing the iter-4
bricking sequence (lock plan → strip `plan.task_ids` → re-run `plan
check`) now backfills `task_ids` and bumps revision by exactly one,
then remains idempotent on the next run. Events file gets exactly one
additional `plan.checked` record from the backfill. Same flow that
iter-4 left bricked now completes cleanly.

Findings: **0 P0 · 0 P1 · 0 P2**.

## Baseline + iter fix verification

| Finding | Severity (orig) | Fix commit | Verified here | Result |
|---|---|---|---|---|
| F1 — `status.next_action` vs `task next` post-AwaitingReview | P1 | 6473650 | §W22 (rev 10: both say `run_review T4`) | FIXED |
| F2 — `status.review_required` stale after clean | P1 | 6473650 | §W26 (rev 12: `review_required: []`) | FIXED |
| F3 (baseline) — clean review creates TaskRecord for review task | P1 | 6473650 | §W27 (`state.tasks.T4.status = complete`) | FIXED |
| F4 — packet field `target_proofs` → `proofs` | P1 | 6473650 | §W24 (`proofs: [...]`) | FIXED |
| F5 — record envelope fields documented | P2 | 6473650 | §W25 (contract table reflects extras) | FIXED |
| F6 — proof path resolution documented | P2 | 6473650 | contract update in `docs/cli-contract-schemas.md` | FIXED (doc only) |
| F7 — CLOSEOUT Tasks table omits T4 | P2 | 6473650 | §W32 (Tasks row "T4 · complete · —") | FIXED |
| F8 — `tasks_complete` consulted wrong source | P0 | 958d2f1 | §F8 below | FIXED |
| F9 — clippy `assigning_clones` in `plan/check.rs` | P1 | 2ef3ce7 | `cargo clippy` clean | FIXED |
| F10 — two status tests failed after F8 landed | P1 | 2ef3ce7 | `cargo test --release` clean (170/170) | FIXED |
| F11 — plan-check upgrade trap (task_ids missing) | P2 → P1 (iter4) | b212ca8 + c5e07ad | §F11 below | FIXED |
| F3 (wave) — `close check` blocker loop walked `state.tasks` | P1 | 5a16894 | §F3 below | FIXED |

## F3 (wave) regression check — partial-completion `close check`

Run directly against the walkthrough mission at revision 6 (T1 complete,
T2/T3/T4 never started, `plan.task_ids = [T1,T2,T3,T4]`):

```shell
$ cd /tmp/codex1-iter5-demo && codex1 --json close check --mission demo
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

Exactly the three DAG-node-missing blockers promised by the 5a16894
fix. Matching `status` at the same revision (§W17 below) returns
`verdict: continue_required` and `close_ready: false`.

## F8 regression check — partial-completion `status`

Same state as above (revision 6, T1 complete, T2/T3/T4 never started):

```shell
$ codex1 --json status --mission demo
{ "ok": true, "revision": 6,
  "data": { "verdict": "continue_required", "close_ready": false,
            "next_action": { "kind": "run_wave", "tasks": ["T2","T3"], "wave_id": "W2" },
            "ready_tasks": ["T2","T3"], ... } }
```

Verdict is `continue_required`, not `ready_for_mission_close_review`.
F8 FIXED.

## F11 upgrade-trap check — FIXED

Source guard now present at `crates/codex1/src/cli/plan/check.rs:69-72`:

```rust
let current = state::load(&paths)?;
let hash_matches = current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());
let task_ids_missing = current.plan.task_ids.is_empty();
let already_locked_same = hash_matches && !task_ids_missing;
```

Reproducible sequence in `/tmp/codex1-iter5-f11`:

**Step 1** — init → ratify → choose-level → scaffold → first `plan check`:
```shell
$ codex1 --json plan check --mission f11
{ "ok": true, "revision": 4, "data": { "locked": true, ... } }
$ jq '{revision, plan: {task_ids: .plan.task_ids}}' PLANS/f11/STATE.json
{ "revision": 4, "plan": { "task_ids": ["T1","T2","T3","T4"] } }
```

**Step 2** — simulate pre-F8 binary by dropping `plan.task_ids`:
```python
s = json.load(open('PLANS/f11/STATE.json'))
s['plan'].pop('task_ids', None)
json.dump(s, open('PLANS/f11/STATE.json','w'), indent=2)
```

State now has `plan.task_ids = null` but `plan.locked = true` and
`plan.hash` unchanged. This is exactly the iter-4 F11 reproducer.

**Step 3** — re-run `plan check` (backfill run):
```shell
$ codex1 --json plan check --mission f11
{ "ok": true, "revision": 5, "data": { "locked": true,
  "plan_hash": "sha256:bc6066688927e5d4058ef12c3a7682eba1c0a84e211a45136221b495acdde3cc",
  "review_tasks": 1, "tasks": 4, "hard_evidence": 0 } }

$ jq '{revision, plan: {task_ids: .plan.task_ids}}' PLANS/f11/STATE.json
{ "revision": 5, "plan": { "task_ids": ["T1","T2","T3","T4"] } }
```

Revision bumped 4 → 5, `task_ids` restored to full 4-entry list.

**Step 4** — re-run again (idempotency):
```shell
$ codex1 --json plan check --mission f11
{ "ok": true, "revision": 5, "data": { "locked": true, ... } }  # unchanged

$ jq '.revision' PLANS/f11/STATE.json
5                                              # unchanged
```

Revision stays at 5 on the second re-run. The guard fires exactly once.

**Step 5** — events file shows exactly one additional `plan.checked`:
```shell
$ cat PLANS/f11/EVENTS.jsonl | jq -c '{seq, kind}'
{"seq":1,"kind":"outcome.ratified"}
{"seq":2,"kind":"plan.choose_level"}
{"seq":3,"kind":"plan.scaffold"}
{"seq":4,"kind":"plan.checked"}        # original lock
{"seq":5,"kind":"plan.checked"}        # backfill
# no seq 6 — idempotent re-run did not append
```

Five events total, with the backfill contributing exactly one new
`plan.checked` entry and the second idempotent run contributing none.
F11 is now fully repaired at source (b212ca8) and guarded by test
(`plan_check_backfills_missing_task_ids_and_then_stays_idempotent`
in `crates/codex1/tests/plan_check.rs:658-710`, added in c5e07ad).

## Walkthrough transcript (condensed)

All envelopes captured against `/tmp/codex1-iter5-demo`, mission `demo`.

### W1. Install

```shell
$ cd /Users/joel/codex1/.claude/worktrees/agent-aec89e1f && time make install-local
   <snip: ~65 release compile lines>
    Finished `release` profile [optimized] target(s) in 22.94s
cp target/release/codex1 /Users/joel/.local/bin/codex1
Installed codex1 to /Users/joel/.local/bin/codex1
```

### W2. Verify from /tmp

```shell
$ cd /tmp && command -v codex1
/Users/joel/.local/bin/codex1
$ codex1 --version
codex1 0.1.0
$ codex1 --help        # 11 subcommands: init, doctor, hook, outcome,
                       # plan, task, review, replan, loop, close, status
```

### W3. Doctor (from /tmp)

```shell
$ cd /tmp && codex1 --json doctor
{ "ok": true, "data": {
    "auth": { "required": false,
              "notes": "Codex1 is a local mission harness; no auth is required by default." },
    "config": { "exists": false, "path": "/Users/joel/.codex1/config.toml" },
    "cwd": "/private/tmp",
    "install": { "codex1_on_path": "/Users/joel/.local/bin/codex1",
                 "home_local_bin_writable": true,
                 "home_local_bin": "/Users/joel/.local/bin" },
    "version": "0.1.0", "warnings": [] } }
```

### W4. Init

```shell
$ cd /tmp/codex1-iter5-demo && codex1 --json init --mission demo
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
            "ratified_at": "2026-04-21T08:07:36.877008Z" } }
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
→ T4 review of [T2,T3]) and SPEC.md files were written under
`PLANS/demo/specs/T{1..4}/`.

### W11. Plan check

```shell
$ codex1 --json plan check --mission demo
{ "ok": true, "revision": 4,
  "data": { "hard_evidence": 0, "locked": true,
            "plan_hash": "sha256:6d29c21ba6067cac8b4a3f2361dfa085a687732a719944ec4d2d16b81fad4225",
            "review_tasks": 1, "tasks": 4 } }
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
  "data": { "idempotent": false, "started_at": "2026-04-21T08:08:18.249406Z",
            "status": "in_progress", "task_id": "T1" } }
```

### W16. Task finish T1

```shell
$ codex1 --json task finish T1 --mission demo --proof specs/T1/PROOF.md
{ "ok": true, "revision": 6,
  "data": { "finished_at": "2026-04-21T08:08:22.933971Z",
            "proof_path": "specs/T1/PROOF.md",
            "status": "complete", "task_id": "T1" } }
```

(T1 has no `review_target`, so it transitions directly to `complete`.)

### W17. F3+F8 regression sampling — revision 6

State snapshotted at revision 6 (T1 complete, T2/T3/T4 never started).
See §F3 and §F8 sections above for full envelopes; both pass. Status
at revision 6 returned `verdict=continue_required`, `close_ready=false`,
`ready_tasks=["T2","T3"]`, `next_action={kind: run_wave, wave_id: W2,
tasks: [T2,T3]}` — full agreement with `task next` and `close check`.

### W18. Task start T2

```shell
$ codex1 --json task start T2 --mission demo
{ "ok": true, "revision": 7,
  "data": { "idempotent": false, "started_at": "2026-04-21T08:08:33.927953Z",
            "status": "in_progress", "task_id": "T2" } }
```

### W19. Task finish T2

```shell
$ codex1 --json task finish T2 --mission demo --proof specs/T2/PROOF.md
{ "ok": true, "revision": 8,
  "data": { "finished_at": "2026-04-21T08:08:38.516973Z",
            "proof_path": "specs/T2/PROOF.md",
            "status": "awaiting_review", "task_id": "T2" } }
```

### W20. Task start T3

```shell
$ codex1 --json task start T3 --mission demo
{ "ok": true, "revision": 9,
  "data": { "idempotent": false, "started_at": "2026-04-21T08:08:39.149077Z",
            "status": "in_progress", "task_id": "T3" } }
```

### W21. Task finish T3

```shell
$ codex1 --json task finish T3 --mission demo --proof specs/T3/PROOF.md
{ "ok": true, "revision": 10,
  "data": { "finished_at": "2026-04-21T08:08:42.839845Z",
            "proof_path": "specs/T3/PROOF.md",
            "status": "awaiting_review", "task_id": "T3" } }
```

### W22. F1 verification — status vs `task next` post-AwaitingReview

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

### W23. Review start T4

```shell
$ codex1 --json review start T4 --mission demo
{ "ok": true, "revision": 11,
  "data": { "boundary_revision": 11, "review_task_id": "T4",
            "targets": ["T2","T3"], "verdict": "pending" } }
```

### W24. Review packet T4 (F4 verification)

```shell
$ codex1 --json review packet T4 --mission demo
{ "ok": true, "revision": 11,
  "data": {
    "task_id": "T4", "targets": ["T2","T3"], "review_profile": "code_bug_correctness",
    "diffs": [],
    "proofs": [ "PLANS/demo/specs/T2/PROOF.md", "PLANS/demo/specs/T3/PROOF.md" ],
    "mission_id": "demo", "mission_summary": "|\nA ratified OUTCOME.md, a locked 4-task PLAN.yaml ...",
    "profiles": [], "reviewer_instructions": "You are a Codex1 reviewer. Do not edit files. ...",
    "target_specs": [
      { "task_id": "T2", "spec_path": "specs/T2/SPEC.md", "spec_excerpt": "# T2 — Parallel branch A ..." },
      { "task_id": "T3", "spec_path": "specs/T3/SPEC.md", "spec_excerpt": "# T3 — Parallel branch B ..." } ] } }
```

`proofs` uses the canonical contract name. F4 FIXED.

### W25. Review record T4 --clean (F5)

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

### W26. F2 + baseline-F3 verification

```shell
$ codex1 --json status --mission demo
{ ... "revision": 12, "verdict": "ready_for_mission_close_review",
       "close_ready": false, "ready_tasks": [],
       "review_required": [],               # F2: empty, not stale with T4
       "next_action": { "command": "$review-loop (mission-close)",
                        "kind": "mission_close_review",
                        "hint": "All tasks complete; run the mission-close review." } }

$ jq '.tasks.T4' PLANS/demo/STATE.json
{ "id": "T4", "status": "complete",
  "finished_at": "2026-04-21T08:08:55.433982Z", "superseded_by": null }
```

F2 and baseline-F3 FIXED.

### W27. Task next after T4 clean

```shell
$ codex1 --json task next --mission demo
{ "ok": true, "revision": 12,
  "data": { "next": { "kind": "mission_close_review",
                      "reason": "all tasks complete or superseded" } } }
```

(Previously in baseline this still returned `run_task T4` — FIXED.)

### W28. Close check before mission-close review

```shell
$ codex1 --json close check --mission demo
{ "ok": true, "revision": 12,
  "data": { "blockers": [ { "code": "CLOSE_NOT_READY",
                            "detail": "mission-close review has not started" } ],
            "ready": false, "verdict": "ready_for_mission_close_review" } }
```

`status` and `close check` agree on `close_ready: false` with verdict
`ready_for_mission_close_review`.

### W29. Close record-review --clean

```shell
$ codex1 --json close record-review --clean --reviewers mission-close-auditor --mission demo
{ "ok": true, "revision": 13,
  "data": { "consecutive_dirty": 0, "dry_run": false, "findings_file": null,
            "replan_triggered": false, "review_state": "passed",
            "reviewers": ["mission-close-auditor"], "verdict": "clean" } }
```

### W30. Status after mission-close review

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

### W31. Loop activate / pause / resume / deactivate

```shell
$ codex1 --json loop activate --mode execute --mission demo
{ "ok": true, "revision": 14, "data": { "active": true, "mode": "execute",
                                        "paused": false, ... } }
# status: stop = { allow: true, reason: idle,
#                  message: "Loop is active but verdict allows stop." }

$ codex1 --json loop pause --mission demo
{ "ok": true, "revision": 15, "data": { "active": true, "paused": true, ... } }
# status: stop = { allow: true, reason: paused,
#                  message: "Loop is paused; stop is allowed. Use $execute or loop resume to continue." }

$ codex1 --json loop resume --mission demo
{ "ok": true, "revision": 16, "data": { "active": true, "paused": false, ... } }

$ codex1 --json loop deactivate --mission demo
{ "ok": true, "revision": 17, "data": { "active": false, "mode": "none", ... } }
```

### W32. Close complete (terminal)

```shell
$ codex1 --json close complete --mission demo
{ "ok": true, "revision": 18,
  "data": { "closeout_path": ".../CLOSEOUT.md", "dry_run": false,
            "mission_id": "demo", "terminal_at": "2026-04-21T08:09:27.597545Z" } }
```

### W33. Terminal status

```shell
$ codex1 --json status --mission demo
{ ... "revision": 18, "phase": "terminal", "verdict": "terminal_complete",
       "close_ready": false,
       "stop": { "allow": true, "reason": "terminal", ... },
       "next_action": { "kind": "closed", "hint": "Mission is terminal." } }
```

### W34. Close complete idempotency

```shell
$ codex1 --json close complete --mission demo
{ "ok": false, "code": "TERMINAL_ALREADY_COMPLETE",
  "message": "Mission is already terminal (closed at 2026-04-21T08:09:27.597545Z)",
  "hint": "Start a new mission; a terminal mission cannot be reopened.",
  "retryable": false,
  "context": { "closed_at": "2026-04-21T08:09:27.597545Z" } }
```

Contract-documented error code with closed-at context.

### W35. CLOSEOUT.md

```markdown
# CLOSEOUT — demo

**Terminal at:** 2026-04-21T08:09:27.597545Z
**Final revision:** 18
**Planning level:** medium

## Outcome

A ratified OUTCOME.md, a locked 4-task PLAN.yaml (T1 root, T2/T3
fan-out, T4 review of T2 and T3), a full task lifecycle with PROOFs,
a mission-close review, and a terminal CLOSEOUT.md citing every
task id.

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
| T4 | clean | e2e-auditor | — |
| MC | clean | — | — |

## Mission-close review

Clean on the first round.
```

All four task IDs (T1, T2, T3, T4) appear in the Tasks table. F7 FIXED.

### W36. EVENTS.jsonl — append-only, monotonic

```text
{"seq":1,"at":"2026-04-21T08:07:36.898103Z","kind":"outcome.ratified",...}
{"seq":2,"at":"2026-04-21T08:07:41.128070Z","kind":"plan.choose_level",...}
{"seq":3,"at":"2026-04-21T08:07:41.779088Z","kind":"plan.scaffold",...}
{"seq":4,"at":"2026-04-21T08:08:11.156468Z","kind":"plan.checked",...}
{"seq":5,"at":"2026-04-21T08:08:18.254120Z","kind":"task.started",...}      # T1
{"seq":6,"at":"2026-04-21T08:08:22.939494Z","kind":"task.finished",...}     # T1
{"seq":7,"at":"2026-04-21T08:08:33.934590Z","kind":"task.started",...}      # T2
{"seq":8,"at":"2026-04-21T08:08:38.523614Z","kind":"task.finished",...}     # T2
{"seq":9,"at":"2026-04-21T08:08:39.154856Z","kind":"task.started",...}      # T3
{"seq":10,"at":"2026-04-21T08:08:42.845769Z","kind":"task.finished",...}    # T3
{"seq":11,"at":"2026-04-21T08:08:50.469461Z","kind":"review.started",...}   # T4
{"seq":12,"at":"2026-04-21T08:08:55.442430Z","kind":"review.recorded.clean",...}
{"seq":13,"at":"2026-04-21T08:09:07.628082Z","kind":"close.review.clean",...}
{"seq":14,"at":"2026-04-21T08:09:16.144874Z","kind":"loop.activated",...}
{"seq":15,"at":"2026-04-21T08:09:20.327099Z","kind":"loop.paused",...}
{"seq":16,"at":"2026-04-21T08:09:24.087178Z","kind":"loop.resumed",...}
{"seq":17,"at":"2026-04-21T08:09:24.098919Z","kind":"loop.deactivated",...}
{"seq":18,"at":"2026-04-21T08:09:27.605922Z","kind":"close.complete",...}
```

18 events; `seq` strictly increasing; timestamps chronological.

### W37. Ralph hook manual test

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
| loop resumed (rev 16) | true | false | mission_close_review_passed | true | idle |
| loop deactivated (rev 17) | false | false | mission_close_review_passed | true | idle |
| terminal (rev 18) | false | false | terminal_complete | true | terminal |

## Findings

**No findings.** 0 P0 · 0 P1 · 0 P2.

The iter-4 audit promoted F11 from its original P2 to P1 because the
claimed fix was missing from the 2ef3ce7 diff. Commit b212ca8 added the
correct guard (`!task_ids_missing` in the `already_locked_same`
predicate), and commit c5e07ad added a regression test that exercises
the exact iter-4 reproducer. Both live in source at c5e07ad and the
empirical walkthrough in §F11 above confirms the guard fires, bumps
revision, backfills `task_ids`, appends exactly one event, and then
stays idempotent.

The iter-4 concern that the commit history was misleading is also
resolved: b212ca8's commit message accurately describes the landed
change, and c5e07ad's message accurately describes a regression-only
follow-up. The test harness's "170 passing" comment now reflects the
actual count.

## Clean checks

- [x] `codex1 --help` lists every documented command (11 subcommands:
  init, doctor, hook, outcome, plan, task, review, replan, loop,
  close, status — matches the contract-referenced surface).
- [x] `codex1 --json doctor` returns `ok: true` without auth
  (`required: false`).
- [x] Full mission reaches `terminal_complete` (status at revision 18).
- [x] `CLOSEOUT.md` is written and cites every task ID (T1, T2, T3, T4
  all present in the Tasks table).
- [x] `EVENTS.jsonl` is append-only and monotonic (seq 1..18;
  timestamps chronological).
- [x] `codex1 status` and `codex1 close check` agree on `close_ready`
  at every captured phase (see §W28, §W30, §F3, §F8 transcript).
- [x] Ralph hook shell exits 0 when `stop.allow: true`, 2 when false
  (§W37).
- [x] F3 (wave) close-check blocker enumeration walks `plan.task_ids`
  (§F3 — three `TASK_NOT_READY` blockers at rev 6).
- [x] F8 status verdict computation consults `plan.task_ids` (§F8 —
  `continue_required`, not `ready_for_mission_close_review`, at rev 6).
- [x] F11 `plan check` backfills `task_ids` on upgraded STATE.json
  (§F11 — rev 4→5 with restored task_ids).
- [x] F11 second `plan check` after backfill is idempotent (§F11 —
  revision stays at 5; no additional event).
- [x] F11 regression test present and passes
  (`plan_check_backfills_missing_task_ids_and_then_stays_idempotent`
  in `crates/codex1/tests/plan_check.rs`).
- [x] `cargo fmt --check` clean.
- [x] `cargo clippy --all-targets -- -D warnings` clean.
- [x] `cargo test --release` all 170 tests pass.

All clean checks pass. Zero findings at P0, P1, or P2. Codex1 v3 at
c5e07ad is production-ready for the happy path, the documented
regression scenarios, and the post-upgrade recovery scenario.
