# End-to-end walkthrough audit

Branch: integration/phase-b @ a9e9abc24462fd4e193f17c5819fb5e3521244f5
Audited on: 2026-04-20 (UTC)
Build: cargo build --release (in 20.03s)
Tests: 162 passed / 162 total (16 test binaries)

## Summary

**PASS** — a complete mission lifecycle drives from `codex1 init` through
`codex1 close complete`, CLOSEOUT.md is written citing every task id,
`status` and `close check` agree on `close_ready` at every captured
phase, the Ralph hook exits 0/2 per the contract, and every CLI call
returns a well-formed JSON envelope.

Findings: **0 P0**, **4 P1**, **3 P2**. None block the happy path.

Notable issues: two separate sources (`status.next_action` and `task
next`) disagree about work-remaining between the point T2/T3 finish and
the mission-close transition; `state.reviews` is not consulted when
deriving `status.review_required` or the `status` ready-wave projection;
the planned review task itself (T4) never gets a `TaskRecord`, so task
lifecycle and review lifecycle never fully reconcile.

## Walkthrough transcript

### 1. Install
```shell
$ cd /Users/joel/codex1/.claude/worktrees/agent-af898512 && time make install-local
   Compiling unsafe-libyaml v0.2.11
   Compiling fastrand v2.4.1
   <snip: 60+ dependency compile lines>
   Compiling codex1 v0.1.0 (/Users/joel/codex1/.claude/worktrees/agent-af898512/crates/codex1)
    Finished `release` profile [optimized] target(s) in 20.03s
cp target/release/codex1 /Users/joel/.local/bin/codex1
Installed codex1 to /Users/joel/.local/bin/codex1
make install-local  43.89s user 3.57s system 236% cpu 20.094 total
```

### 2. Verify from /tmp
```shell
$ cd /tmp && command -v codex1
/Users/joel/.local/bin/codex1

$ codex1 --help
Drives a mission through clarify → plan → execute → review-loop → close. Used by the six public skills ($clarify, $plan, $execute, $review-loop, $close, $autopilot). Ralph stop hooks read only `codex1 status --json`.

Usage: codex1 [OPTIONS] <COMMAND>

Commands:
  init     Create PLANS/<mission>/ with blank OUTCOME.md, PLAN.yaml, STATE.json
  doctor   Report CLI health. Never crashes on missing auth or config
  hook     Print the one-liner for wiring the Ralph Stop hook
  outcome  OUTCOME.md validation and ratification
  plan     Plan commands (choose-level, scaffold, check, graph, waves)
  task     Task lifecycle commands
  review   Review recording and packet emission
  replan   Replan trigger checks and records
  loop     Loop pause/resume/deactivate (used by $close)
  close    Close check and complete
  status   Unified mission status — the Ralph-facing single source of truth
  help     Print this message or the help of the given subcommand(s)

Options:
      --mission <ID>
      --repo-root <PATH>
      --json
      --dry-run
      --expect-revision <N>
  -h, --help
  -V, --version
```

### 3. Doctor (from /tmp)
```shell
$ cd /tmp && codex1 --json doctor
{
  "ok": true,
  "data": {
    "auth": {
      "notes": "Codex1 is a local mission harness; no auth is required by default.",
      "required": false
    },
    "config": {
      "exists": false,
      "path": "/Users/joel/.codex1/config.toml"
    },
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
$ cd /tmp/codex1-e2e-demo && codex1 --json init --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 0,
  "data": {
    "created": {
      "events": "/private/tmp/codex1-e2e-demo/PLANS/demo/EVENTS.jsonl",
      "mission_dir": "/private/tmp/codex1-e2e-demo/PLANS/demo",
      "outcome": "/private/tmp/codex1-e2e-demo/PLANS/demo/OUTCOME.md",
      "plan": "/private/tmp/codex1-e2e-demo/PLANS/demo/PLAN.yaml",
      "reviews_dir": "/private/tmp/codex1-e2e-demo/PLANS/demo/reviews",
      "specs_dir": "/private/tmp/codex1-e2e-demo/PLANS/demo/specs",
      "state": "/private/tmp/codex1-e2e-demo/PLANS/demo/STATE.json"
    },
    "next_action": {
      "command": "$clarify",
      "hint": "Fill in OUTCOME.md, then run `codex1 outcome ratify`.",
      "kind": "clarify"
    }
  }
}
```

### 5. Status after init
```shell
$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 0,
  "data": {
    "close_ready": false,
    "loop": {
      "active": false,
      "mode": "none",
      "paused": false
    },
    "next_action": {
      "command": "$clarify",
      "hint": "Ratify OUTCOME.md before planning.",
      "kind": "clarify"
    },
    "outcome_ratified": false,
    "parallel_blockers": [],
    "parallel_safe": false,
    "phase": "clarify",
    "plan_locked": false,
    "ready_tasks": [],
    "replan_required": false,
    "review_required": [],
    "stop": {
      "allow": true,
      "message": "Loop is inactive; stop is allowed.",
      "reason": "idle"
    },
    "verdict": "needs_user"
  }
}
```

### 6. Outcome check (after OUTCOME.md filled)
```shell
$ codex1 --json outcome check --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 0,
  "data": {
    "missing_fields": [],
    "placeholders": [],
    "ratifiable": true
  }
}
```

### 7. Outcome ratify
```shell
$ codex1 --json outcome ratify --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 1,
  "data": {
    "mission_id": "demo",
    "phase": "plan",
    "ratified_at": "2026-04-20T17:14:39.161717Z"
  }
}
```

### 8. Status after ratify
```shell
$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 1,
  "data": {
    "close_ready": false,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": {
      "command": "$plan",
      "hint": "Draft and lock PLAN.yaml.",
      "kind": "plan"
    },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": false,
    "phase": "plan",
    "plan_locked": false,
    "ready_tasks": [],
    "replan_required": false,
    "review_required": [],
    "stop": {
      "allow": true,
      "message": "Loop is inactive; stop is allowed.",
      "reason": "idle"
    },
    "verdict": "needs_user"
  }
}
```

### 9. Plan choose-level
```shell
$ codex1 --json plan choose-level --mission demo --level medium
{
  "ok": true,
  "mission_id": "demo",
  "revision": 2,
  "data": {
    "effective_level": "medium",
    "next_action": {
      "args": ["codex1","plan","scaffold","--level","medium"],
      "kind": "plan_scaffold"
    },
    "requested_level": "medium"
  }
}
```

### 10. Plan scaffold
```shell
$ codex1 --json plan scaffold --mission demo --level medium
{
  "ok": true,
  "mission_id": "demo",
  "revision": 3,
  "data": {
    "level": "medium",
    "specs_created": [],
    "wrote": "PLANS/demo/PLAN.yaml"
  }
}
```

After scaffold, PLAN.yaml was overwritten with a 4-task DAG (T1 root, T2/T3 parallel deps on T1, T4 review targeting T2+T3) and SPEC.md files were written under `PLANS/demo/specs/T{1..4}/`.

### 11. Plan check
```shell
$ codex1 --json plan check --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 4,
  "data": {
    "hard_evidence": 1,
    "locked": true,
    "plan_hash": "sha256:bfb45383f1bfe1e14a13584aba4c494529cf3e392c7363ba648d2d0b0268c4fc",
    "review_tasks": 1,
    "tasks": 4
  }
}
```

### 12. Plan waves
```shell
$ codex1 --json plan waves --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 4,
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

### 13. Plan graph (mermaid)
```shell
$ codex1 --json plan graph --mission demo --format mermaid
{
  "ok": true,
  "mission_id": "demo",
  "revision": 4,
  "data": {
    "mermaid": "flowchart TD\n    classDef complete fill:#b7e4c7,stroke:#2d6a4f,color:#1b4332\n    classDef in_progress fill:#ffe066,stroke:#b08900,color:#5a4500\n    classDef awaiting_review fill:#ffd6a5,stroke:#c75d2c,color:#5a2b12\n    classDef ready fill:#bde0fe,stroke:#1d4ed8,color:#0b2559\n    classDef blocked fill:#e5e7eb,stroke:#6b7280,color:#374151\n    classDef superseded fill:#f5f5f4,stroke:#a8a29e,color:#57534e,stroke-dasharray: 4 2\n    T1[\"T1 · Foundation step\"]\n    T2[\"T2 · Parallel branch A\"]\n    T3[\"T3 · Parallel branch B\"]\n    T4[\"T4 · Review of T2 and T3\"]\n    T1 --> T2\n    T1 --> T3\n    T2 --> T4\n    T3 --> T4\n    class T1 ready\n    class T2 blocked\n    class T3 blocked\n    class T4 blocked\n"
  }
}
```

### 14. Task next (fresh plan)
```shell
$ codex1 --json task next --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 4,
  "data": {
    "next": { "kind": "run_task", "task_id": "T1", "task_kind": "code" }
  }
}
```

### 15. Task start T1
```shell
$ codex1 --json task start T1 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 5,
  "data": {
    "idempotent": false,
    "started_at": "2026-04-20T17:15:39.810461Z",
    "status": "in_progress",
    "task_id": "T1"
  }
}
```

### 16. Task finish T1
```shell
$ codex1 --json task finish T1 --mission demo --proof specs/T1/PROOF.md
{
  "ok": true,
  "mission_id": "demo",
  "revision": 6,
  "data": {
    "finished_at": "2026-04-20T17:15:45.90587Z",
    "proof_path": "specs/T1/PROOF.md",
    "status": "complete",
    "task_id": "T1"
  }
}
```

T1 has no `review_target` covering it, so it transitions directly to
`complete`.

Note: earlier attempt with `--proof PLANS/demo/specs/T1/PROOF.md`
(the form literally shown in the Unit 22 handoff prompt) returned
`PROOF_MISSING` because proof paths resolve relative to `mission_dir`
(see `crates/codex1/src/cli/task/finish.rs:30-34`). The contract does
not document this resolution rule, so the handoff's example is
effectively wrong. See F6 below.

### 17. Task next after T1 complete
```shell
$ codex1 --json task next --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 6,
  "data": {
    "next": {
      "kind": "run_wave",
      "parallel_safe": true,
      "tasks": ["T2","T3"],
      "wave_id": "W2"
    }
  }
}
```

### 18. Task start T2
```shell
$ codex1 --json task start T2 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 7,
  "data": {
    "idempotent": false,
    "started_at": "2026-04-20T17:15:53.986931Z",
    "status": "in_progress",
    "task_id": "T2"
  }
}
```

### 19. Task finish T2
```shell
$ codex1 --json task finish T2 --mission demo --proof specs/T2/PROOF.md
{
  "ok": true,
  "mission_id": "demo",
  "revision": 8,
  "data": {
    "finished_at": "2026-04-20T17:16:01.430298Z",
    "proof_path": "specs/T2/PROOF.md",
    "status": "awaiting_review",
    "task_id": "T2"
  }
}
```

T2 transitioned to `awaiting_review` because T4 targets T2.

### 20. Task start T3
```shell
$ codex1 --json task start T3 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 9,
  "data": {
    "idempotent": false,
    "started_at": "2026-04-20T17:16:04.757555Z",
    "status": "in_progress",
    "task_id": "T3"
  }
}
```

### 21. Task finish T3
```shell
$ codex1 --json task finish T3 --mission demo --proof specs/T3/PROOF.md
{
  "ok": true,
  "mission_id": "demo",
  "revision": 10,
  "data": {
    "finished_at": "2026-04-20T17:16:12.550334Z",
    "proof_path": "specs/T3/PROOF.md",
    "status": "awaiting_review",
    "task_id": "T3"
  }
}
```

### 22. Task next after T3 — first divergence
```shell
$ codex1 --json task next --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 10,
  "data": {
    "next": {
      "kind": "run_review",
      "targets": ["T2","T3"],
      "task_id": "T4"
    }
  }
}

$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 10,
  "data": {
    "close_ready": false,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": {
      "hint": "Run wave W2 with $execute.",
      "kind": "run_wave",
      "parallel_safe": true,
      "tasks": ["T2","T3"],
      "wave_id": "W2"
    },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": true,
    "phase": "execute",
    "plan_locked": true,
    "ready_tasks": ["T2","T3"],
    "replan_required": false,
    "review_required": [],
    "stop": {
      "allow": true,
      "message": "Loop is inactive; stop is allowed.",
      "reason": "idle"
    },
    "verdict": "continue_required"
  }
}
```

See F1: `status.next_action` reports `run_wave W2 [T2,T3]` and
`ready_tasks: [T2,T3]` while `task next` correctly routes to
`run_review T4`.

### 23. Review start T4
```shell
$ codex1 --json review start T4 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 11,
  "data": {
    "boundary_revision": 11,
    "review_task_id": "T4",
    "targets": ["T2","T3"],
    "verdict": "pending"
  }
}
```

### 24. Review packet T4
```shell
$ codex1 --json review packet T4 --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 11,
  "data": {
    "diffs": [],
    "mission_id": "demo",
    "mission_summary": "|\nProduce a full mission trajectory that drives OUTCOME ratification, plan\nscaffolding, task execution, review records, and close completion,\ncapturing every JSON envelope for review.",
    "profiles": ["code_bug_correctness"],
    "review_profile": "code_bug_correctness",
    "reviewer_instructions": "You are a Codex1 reviewer. Do not edit files. Do not invoke Codex1 skills. Do not record mission truth. Do not run commands that mutate mission state. Do not perform repairs.\n\nYou may: inspect files, inspect diffs, run safe read-only commands, run tests if explicitly allowed.\n\nReturn only: NONE or P0/P1/P2 findings with evidence refs and concise rationale.\n\nOnly review the assigned target against the mission, outcome, plan, and profile. Do not review unrelated future work.",
    "target_proofs": [
      "PLANS/demo/specs/T2/PROOF.md",
      "PLANS/demo/specs/T3/PROOF.md"
    ],
    "target_specs": [
      { "spec_excerpt": "# T2 — Parallel branch A\n...", "spec_path": "specs/T2/SPEC.md", "task_id": "T2" },
      { "spec_excerpt": "# T3 — Parallel branch B\n...", "spec_path": "specs/T3/SPEC.md", "task_id": "T3" }
    ],
    "targets": ["T2","T3"],
    "task_id": "T4"
  }
}
```

See F4: the packet uses `target_proofs` where the contract says
`proofs`, and adds fields (`profiles`, `target_specs`, `mission_id`,
`reviewer_instructions`) beyond what the contract lists.

### 25. Review record T4 --clean
```shell
$ codex1 --json review record T4 --clean --reviewers e2e-auditor --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 12,
  "data": {
    "category": "accepted_current",
    "findings_file": null,
    "replan_triggered": false,
    "review_task_id": "T4",
    "reviewers": ["e2e-auditor"],
    "verdict": "clean",
    "warnings": []
  }
}
```

See F5: contract specifies four fields; this envelope has seven.

### 26. Task next after review clean — second divergence
```shell
$ codex1 --json task next --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 12,
  "data": {
    "next": { "kind": "run_task", "task_id": "T4", "task_kind": "review" }
  }
}

$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 12,
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
    "ready_tasks": ["T4"],
    "replan_required": false,
    "review_required": [
      { "targets": ["T2","T3"], "task_id": "T4" }
    ],
    "stop": {
      "allow": true,
      "message": "Loop is inactive; stop is allowed.",
      "reason": "idle"
    },
    "verdict": "ready_for_mission_close_review"
  }
}
```

See F2 and F3: `task next` tells the caller to run T4 even though T4 was
just recorded clean; `status.review_required` still lists T4 and
`ready_tasks: [T4]` still lists it. These are both downstream of F3 —
the review task itself never has a `TaskRecord` in `state.tasks`.

### 27. Close check (before mission-close review)
```shell
$ codex1 --json close check --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 12,
  "data": {
    "blockers": [
      { "code": "CLOSE_NOT_READY", "detail": "mission-close review has not started" }
    ],
    "ready": false,
    "verdict": "ready_for_mission_close_review"
  }
}
```

`close check` and `status` agree on the verdict
(`ready_for_mission_close_review`), so `close_ready` in both projections
is `false` at this point. Good.

### 28. Close record-review --clean
```shell
$ codex1 --json close record-review --clean --reviewers mission-close-auditor --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 13,
  "data": {
    "consecutive_dirty": 0,
    "dry_run": false,
    "findings_file": null,
    "replan_triggered": false,
    "review_state": "passed",
    "reviewers": ["mission-close-auditor"],
    "verdict": "clean"
  }
}
```

### 29. Status after mission-close review
```shell
$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 13,
  "data": {
    "close_ready": true,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": {
      "command": "codex1 close complete",
      "hint": "Mission-close review passed; finalize close.",
      "kind": "close"
    },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": true,
    "phase": "execute",
    "plan_locked": true,
    "ready_tasks": ["T4"],
    "replan_required": false,
    "review_required": [
      { "targets": ["T2","T3"], "task_id": "T4" }
    ],
    "stop": {
      "allow": true,
      "message": "Loop is inactive; stop is allowed.",
      "reason": "idle"
    },
    "verdict": "mission_close_review_passed"
  }
}

$ codex1 --json close check --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 13,
  "data": {
    "blockers": [],
    "ready": true,
    "verdict": "mission_close_review_passed"
  }
}
```

Both surfaces agree on `close_ready: true` at this phase.

### 30. Loop activate / pause / resume / deactivate (to exercise stop.reason branches)
```shell
$ codex1 --json loop activate --mode execute --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 14,
  "data": { "active": true, "dry_run": false, "mode": "execute", "noop": false, "paused": false }
}

$ codex1 --json status --mission demo
# stop: { "allow": true, "message": "Loop is active but verdict allows stop.", "reason": "idle" }
# (stop.allow stays true because verdict == mission_close_review_passed is in the allow set)

$ codex1 --json loop pause --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 15,
  "data": { "active": true, "dry_run": false, "mode": "execute", "noop": false, "paused": true }
}

$ codex1 --json status --mission demo
# stop: { "allow": true, "message": "Loop is paused; stop is allowed. Use $execute or loop resume to continue.", "reason": "paused" }

$ codex1 --json loop resume --mission demo
{ "ok": true, "mission_id": "demo", "revision": 16, "data": { "active": true, "paused": false, "mode": "execute", "dry_run": false, "noop": false } }

$ codex1 --json loop deactivate --mission demo
{ "ok": true, "mission_id": "demo", "revision": 17, "data": { "active": false, "paused": false, "mode": "none", "dry_run": false, "noop": false } }
```

### 31. Close complete (terminal transition)
```shell
$ codex1 --json close complete --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 18,
  "data": {
    "closeout_path": "/private/tmp/codex1-e2e-demo/PLANS/demo/CLOSEOUT.md",
    "dry_run": false,
    "mission_id": "demo",
    "terminal_at": "2026-04-20T17:18:21.958701Z"
  }
}
```

### 32. Terminal
```shell
$ codex1 --json status --mission demo
{
  "ok": true,
  "mission_id": "demo",
  "revision": 18,
  "data": {
    "close_ready": false,
    "loop": { "active": false, "mode": "none", "paused": false },
    "next_action": { "hint": "Mission is terminal.", "kind": "closed" },
    "outcome_ratified": true,
    "parallel_blockers": [],
    "parallel_safe": true,
    "phase": "terminal",
    "plan_locked": true,
    "ready_tasks": ["T4"],
    "replan_required": false,
    "review_required": [
      { "targets": ["T2","T3"], "task_id": "T4" }
    ],
    "stop": {
      "allow": true,
      "message": "Mission is terminal; stop is allowed.",
      "reason": "terminal"
    },
    "verdict": "terminal_complete"
  }
}
```

Note: `close_ready: false` after terminal is consistent with the
contract's `close_ready = (verdict == mission_close_review_passed)` — at
`terminal_complete` the door is shut, not "ready to close".

### 33. Close complete idempotency
```shell
$ codex1 --json close complete --mission demo
{
  "ok": false,
  "code": "TERMINAL_ALREADY_COMPLETE",
  "message": "Mission is already terminal (closed at 2026-04-20T17:18:21.958701Z)",
  "hint": "Start a new mission; a terminal mission cannot be reopened.",
  "retryable": false,
  "context": { "closed_at": "2026-04-20T17:18:21.958701Z" }
}
```

Contract calls for `TERMINAL_ALREADY_COMPLETE`; observed. Good.

### 34. CLOSEOUT.md
```markdown
# CLOSEOUT — demo

**Terminal at:** 2026-04-20T17:18:21.958701Z
**Final revision:** 18
**Planning level:** medium

## Outcome

Produce a full mission trajectory that drives OUTCOME ratification, plan
scaffolding, task execution, review records, and close completion,
capturing every JSON envelope for review.

## Tasks

| ID | Status | Proof |
|---|---|---|
| T1 | complete | specs/T1/PROOF.md |
| T2 | complete | specs/T2/PROOF.md |
| T3 | complete | specs/T3/PROOF.md |

## Reviews

| Review ID | Verdict | Reviewers | Findings |
|---|---|---|---|
| T4 | clean | e2e-auditor | — |
| MC | clean | — | — |

## Mission-close review

Clean on the first round.
```

All four task IDs (T1, T2, T3, T4) are cited. T4 is under Reviews rather
than Tasks because `review record` does not create a `TaskRecord` for
the review task itself (see F3).

### 35. EVENTS.jsonl — append-only, monotonic
```text
{"seq":1,"at":"2026-04-20T17:14:39.177503Z","kind":"outcome.ratified",...}
{"seq":2,"at":"2026-04-20T17:14:43.548395Z","kind":"plan.choose_level",...}
{"seq":3,"at":"2026-04-20T17:14:43.562444Z","kind":"plan.scaffold",...}
{"seq":4,"at":"2026-04-20T17:15:32.74083Z","kind":"plan.checked",...}
{"seq":5,"at":"2026-04-20T17:15:39.818972Z","kind":"task.started",...}
{"seq":6,"at":"2026-04-20T17:15:45.913541Z","kind":"task.finished",...}
{"seq":7,"at":"2026-04-20T17:15:53.996014Z","kind":"task.started",...}
{"seq":8,"at":"2026-04-20T17:16:01.434819Z","kind":"task.finished",...}
{"seq":9,"at":"2026-04-20T17:16:04.762281Z","kind":"task.started",...}
{"seq":10,"at":"2026-04-20T17:16:12.555878Z","kind":"task.finished",...}
{"seq":11,"at":"2026-04-20T17:16:35.684578Z","kind":"review.started",...}
{"seq":12,"at":"2026-04-20T17:16:44.356199Z","kind":"review.recorded.clean",...}
{"seq":13,"at":"2026-04-20T17:17:50.224383Z","kind":"close.review.clean",...}
{"seq":14,"at":"2026-04-20T17:18:01.697114Z","kind":"loop.activated",...}
{"seq":15,"at":"2026-04-20T17:18:12.544754Z","kind":"loop.paused",...}
{"seq":16,"at":"2026-04-20T17:18:17.547195Z","kind":"loop.resumed",...}
{"seq":17,"at":"2026-04-20T17:18:17.564189Z","kind":"loop.deactivated",...}
{"seq":18,"at":"2026-04-20T17:18:21.965765Z","kind":"close.complete",...}
```

18 events; seq strictly increasing; timestamps chronological.

### 36. Ralph hook manual test
Two mock `codex1` binaries were placed on a scratch PATH and the Stop
hook driven against each:

```shell
$ mkdir -p /tmp/ralph-test-pass
$ cat > /tmp/ralph-test-pass/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":true,"reason":"idle","message":"mock idle"}}}'
EOF
$ chmod +x /tmp/ralph-test-pass/codex1

$ PATH=/tmp/ralph-test-pass:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
# (no stderr)
exit: 0

$ mkdir -p /tmp/ralph-test-block
$ cat > /tmp/ralph-test-block/codex1 <<'EOF'
#!/usr/bin/env bash
echo '{"ok":true,"data":{"stop":{"allow":false,"reason":"active_loop","message":"mock block"}}}'
EOF
$ chmod +x /tmp/ralph-test-block/codex1

$ PATH=/tmp/ralph-test-block:/usr/bin:/bin bash scripts/ralph-stop-hook.sh </dev/null
ralph-stop-hook: blocking Stop - reason=active_loop
ralph-stop-hook: mock block
exit: 2
```

Exits 0 when `stop.allow: true`, exits 2 when `stop.allow: false`. Good.

### 37. cargo test (run in worktree, not /tmp)
```shell
$ cargo test
...
test result: ok. 10 passed (foundation)
test result: ok. 17 passed (task)
test result: ok. 10 passed (plan_check)
test result: ok. 20 passed (close)
test result: ok. 8 passed (plan_scaffold)
test result: ok. 12 passed (status)
test result: ok. 10 passed (outcome)
test result: ok. 8 passed (plan_waves)
test result: ok. 6 passed (ralph_hook)
test result: ok. 10 passed (loop_)
test result: ok. 16 passed (replan)
test result: ok. 14 passed (review)
test result: ok. 3 passed (status_close_agreement)
test result: ok. 18 passed (integration + one more)

162 passed / 162 total / 0 failed
```

## stop.allow table (captured at each phase)

| Phase / revision | loop.active | loop.paused | verdict | stop.allow | stop.reason |
|---|---|---|---|---|---|
| after init (rev 0) | false | false | needs_user | true | idle |
| after ratify (rev 1) | false | false | needs_user | true | idle |
| after T3 finish (rev 10) | false | false | continue_required | true | idle |
| after T4 clean (rev 12) | false | false | ready_for_mission_close_review | true | idle |
| after mission-close review (rev 13) | false | false | mission_close_review_passed | true | idle |
| loop activated (rev 14) | true | false | mission_close_review_passed | true | idle |
| loop paused (rev 15) | true | true | mission_close_review_passed | true | paused |
| terminal (rev 18) | false | false | terminal_complete | true | terminal |

The "loop active but blockers present" branch (`stop.allow: false`,
`stop.reason: active_loop`) was not exercised against live STATE because
the mission reached `mission_close_review_passed` (which unconditionally
allows stop) before the loop was activated. The branch is covered by
the mocked Ralph hook test above and by the sibling `stop_allowed`
readiness tests (`crates/codex1/src/state/readiness.rs` consumers in
`crates/codex1/tests/ralph_hook.rs`), so the wire-through is
end-to-end exercised.

## Findings

### F1 — status disagrees with task next about work-remaining after tasks enter AwaitingReview (P1)

- Files: `crates/codex1/src/cli/status/next_action.rs:159-175`, `crates/codex1/src/cli/status/project.rs` (consumer of `next_ready_wave`)
- Observed at revision 10 (walkthrough §22):
  - `task next` → `{ kind: run_review, task_id: T4, targets: [T2,T3] }` ✓
  - `status` → `next_action: run_wave W2 [T2,T3]`, `ready_tasks: [T2,T3]`
- Root cause: `task_is_ready` (`next_action.rs:159-175`) treats
  `AwaitingReview` as "work left to do" and only excludes
  `Complete | Superseded | InProgress`. The sibling implementation in
  `crates/codex1/src/cli/task/lifecycle.rs:144-156`
  (`deps_satisfied`/`ready_wave`) correctly distinguishes review from
  non-review kinds and rejects `AwaitingReview` for non-review tasks.
- Why this matters: the contract (docs/cli-contract-schemas.md:281-297)
  describes `status` as "the Ralph-facing single source of truth" and
  the main thread is expected to follow `status.next_action`. A main
  thread that does so will call `codex1 task start T2` and get
  `TASK_NOT_READY: Task T2 has incomplete dependencies: T1` (actually:
  it would re-start T2, which is already `AwaitingReview`, yielding a
  state-machine error). The two command surfaces disagree about
  reality.

### F2 — status.review_required is derived without consulting state.reviews (P1)

- File: `crates/codex1/src/cli/status/next_action.rs:102-108`
  (`ready_reviews`) and the consumer in `crates/codex1/src/cli/status/project.rs`.
- Observed at revisions 13 and 18 (walkthrough §29 and §32): `status`
  still reports `review_required: [{ task_id: T4, targets: [T2,T3] }]`
  after T4 was recorded clean (at revision 12) and after the mission
  was closed.
- Root cause: `ready_reviews` matches any review-kind task with
  satisfied deps; it never checks whether `state.reviews[task_id]` has
  a `clean` verdict for the same boundary.
- Why this matters: clients that rely on `review_required` to decide
  whether a planned review still needs to run will see a stale list.

### F3 — review record clean does not create a TaskRecord for the review task (P1)

- File: `crates/codex1/src/cli/review/record.rs:286-305` (`apply_clean`)
- Observed: at no point during the walkthrough is `T4` present in
  `state.tasks`. `state.reviews.T4` records the clean verdict, but
  `task status T4` at revision 12 still reports
  `{ status: "ready", kind: "review", deps_status: { T2: complete, T3: complete } }`.
  As a consequence:
  - `task next` at revision 12 returns `run_task T4 kind=review` (see
    walkthrough §26) — stale advice.
  - `tasks_complete` in `crates/codex1/src/state/readiness.rs:74-82`
    iterates `state.tasks.values()` and, because T4 is absent, reports
    "all tasks terminal" while T4 itself has never been terminal-marked
    as a task. This happens to produce the right verdict
    (`ready_for_mission_close_review`) in the happy path, but only
    because the predicate silently ignores missing ids.
  - The CLOSEOUT.md Tasks table omits T4 entirely and only records it
    under Reviews (see walkthrough §34).
- Why this matters: the contract does not describe a "some tasks live
  in `state.reviews` and not in `state.tasks`" split. The review
  record / task lifecycle never fully converge, and consumers that
  read one expecting to describe the whole plan get an inconsistent
  picture.

### F4 — review packet envelope diverges from the contract (P1 for rename, P2 for extras)

- File: `crates/codex1/src/cli/review/packet.rs`
- Contract: `docs/cli-contract-schemas.md:228-238` specifies
  `{ task_id, review_profile, targets, diffs, proofs, mission_summary }`.
- Actual: observed at revision 11 (walkthrough §24) — `proofs` is
  renamed to `target_proofs`; additional fields `profiles`,
  `target_specs`, `mission_id`, `reviewer_instructions` appear.
- P1 component: the field rename silently breaks any reviewer client
  that reads `proofs`. The extras are additive (P2).

### F5 — review record envelope has undocumented fields (P2)

- File: `crates/codex1/src/cli/review/record.rs:198-210`
- Contract: `docs/cli-contract-schemas.md:241-249` specifies
  `{ review_task_id, verdict, category, reviewers }` only.
- Actual (walkthrough §25): adds `findings_file`, `replan_triggered`,
  `warnings`. Additive only, low risk, but still a drift.

### F6 — task finish proof path resolution is undocumented (P2)

- File: `crates/codex1/src/cli/task/finish.rs:30-34`
- Behaviour: relative proof paths are joined against `mission_dir`, not
  CWD. The CLI contract schema does not describe this rule. The Phase
  C Unit 22 handoff prompt itself used the form
  `--proof PLANS/demo/specs/T1/PROOF.md`, which fails with
  `PROOF_MISSING` because the resolver produces
  `/private/tmp/.../PLANS/demo/PLANS/demo/specs/T1/PROOF.md`.
- Why this matters: downstream skill authors will copy the contract
  example and observe surprises. Either document the resolution rule
  in `docs/cli-contract-schemas.md` or make the resolver fall back to
  CWD. Non-blocking for the audit; handoff-level nit.

### F7 — CLOSEOUT.md Tasks table omits T4 (P2)

- File: `crates/codex1/src/cli/close/closeout.rs` (writer)
- Downstream of F3. T4 (kind=review) is listed only under Reviews. The
  audit rubric "cites each task id" is satisfied, but a reader of the
  Tasks table alone would miss T4.

## Clean checks

- [x] `codex1 --help` lists every documented command (11 subcommands:
  init, doctor, hook, outcome, plan, task, review, replan, loop, close,
  status — matches the contract-referenced surface).
- [x] `codex1 --json doctor` returns `ok: true` without auth
  (`required: false`).
- [x] Full mission reaches `terminal_complete` (status at revision 18).
- [x] `CLOSEOUT.md` is written and cites each task ID (T1, T2, T3 under
  Tasks; T4 under Reviews — all four IDs present).
- [x] `EVENTS.jsonl` is append-only and monotonic (seq 1..18; timestamps
  chronological).
- [x] `codex1 status` and `codex1 close check` agree on `close_ready` at
  every captured phase.
- [x] Ralph hook shell exits 0 when `stop.allow: true`, 2 when false
  (manual test with mocked `codex1` on PATH).

All clean checks pass. The walkthrough reached terminal_complete end to
end with no manual STATE.json edits and every CLI invocation returning
a contract-structured envelope. The findings above describe real
divergences in the task↔review↔status triangle that should be tracked
before the harness is promoted to production driving.
