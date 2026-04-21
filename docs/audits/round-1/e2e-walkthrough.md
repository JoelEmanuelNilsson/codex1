# Round 1 — e2e-walkthrough audit

## Summary

Drove three missions end-to-end from `/tmp/codex1-review-e2e/` using freshly-built `~/.local/bin/codex1` (v0.1.0, `make -C /Users/joel/codex1 install-local`). Happy path (init → outcome ratify → plan choose-level/scaffold/check → task start/finish T1..T3 → review start/record T4 → close record-review → close check → close complete) succeeded and landed at `verdict: terminal_complete` with `stop.reason: terminal`, `CLOSEOUT.md` written, revision 15, 15 monotonically-sequenced events. Every mutating command bumped `revision` by exactly one; sequence numbers matched final revision. Ralph hook (`scripts/ralph-stop-hook.sh`) exited `2` with `"blocking Stop - reason=active_loop"` on canned `stop.allow:false` and exited `0` on `stop.allow:true` and on missing-codex1.

Failure paths confirmed: `OUTCOME_INCOMPLETE` on placeholders and on `title: TODO`; `TASK_NOT_READY` on start-before-plan; `PROOF_MISSING` on missing proof file; `CLOSE_NOT_READY` when verdict != `mission_close_review_passed`; `TERMINAL_ALREADY_COMPLETE` on second `close complete`; `REPLAN_REQUIRED` after six `review start` + `review record --findings-file` cycles (note: six is per-boundary, not per-record call — this matches `02-cli-contract.md:439-450` review-freshness spec).

Three real findings surfaced: the post-replan task-mutation hole (P1), the plan-check not detecting a review-target cycle that deadlocks execution (P2 combined with its misleading blocked-message), and `ready_tasks` ignoring `plan_locked:false` in the status projection (P2). Two smaller issues listed as P3.

## P0

None.

## P1

### P1-1 — `task start`/`task finish`/`review record` bypass the `plan.locked = false` gate set by `replan record`

**Citation:**
- Contract: `docs/codex1-rebuild-handoff/02-cli-contract.md:131-137` (next_action says `plan`); `02-cli-contract.md:467-469` (replan writes unlock + "new tasks added by editing PLAN.yaml"); `docs/codex1-rebuild-handoff/01-product-flow.md:42-46` (replan → Waves, never back into the same DAG).
- Diverging code: `/Users/joel/codex1/crates/codex1/src/cli/task/start.rs:17-35` (reads plan+deps, no `plan.locked` check); `crates/codex1/src/cli/task/finish.rs:16-53` (no `plan.locked` check); `crates/codex1/src/cli/review/record.rs:38-113` (no `plan.locked` check). Compare `crates/codex1/src/cli/close/check.rs:84` which does gate on `plan.locked`.
- Related: `crates/codex1/src/cli/replan/record.rs:153` sets `state.plan.locked = false`.

**Evidence (reproducible with a `replan record` state):**

```
$ codex1 replan record --supersedes T1 --reason six_dirty --json --mission dirty --repo-root <root>
# → ok:true, plan_locked:false, phase_after:"plan"

$ codex1 status --json --mission dirty --repo-root <root>
# plan_locked:false, verdict:needs_user, next_action:{kind:"plan"}
# ready_tasks:["T2"]   (see P2-2)

$ codex1 task start T2 --json --mission dirty --repo-root <root>
{
  "ok": true,
  "revision": 25,
  "data": { "idempotent": false, "status": "in_progress", "task_id": "T2" }
}

$ codex1 task finish T2 --proof specs/T2/PROOF.md --mission dirty --repo-root <root> --json
{ "ok": true, "revision": 26, "data": { "status": "complete", "task_id": "T2" } }
```

Resulting `STATE.json`: `plan.locked:false`, `replan.triggered:true, triggered_reason:"six_dirty"`, `phase:"plan"`, but `tasks.T2.status:"complete"`. Work-phase mutations accepted while the plan is explicitly being re-planned, with `replan.triggered` still set. If the main thread edits PLAN.yaml after replan, those state mutations may attach to a task whose spec has since changed (or been deleted) — state corruption by contract regression.

**Suggested fix:**

At the top of `task start`, `task finish`, `review start`, and `review record` (and any other non-close, non-replan, non-plan command that mutates state), load `state`, and if `!state.plan.locked` return `PLAN_INVALID` (or a new `PLAN_NOT_LOCKED`) with hint "`Run codex1 plan check` first.". Mirror the check already in `cli/close/check.rs:84` and `cli/plan/waves.rs:66`. Add a regression test in `tests/replan.rs` that asserts `task start` after `replan record` returns a non-OK envelope.

## P2

### P2-1 — `plan check` accepts a plan shape that deadlocks `task next`, and the status error then misdirects

**Citation:**
- Contract: `02-cli-contract.md:338-349` (`plan check` guarantees DAG has no cycle and deps exist, but readiness reachability is not checked); `01-product-flow.md` (expects reviews to flow after work is done).
- Code, plan-check side: `crates/codex1/src/cli/plan/check.rs:247-322` (`validate`/`validate_tasks`/topological check covers cycles and missing deps only).
- Code, readiness-diagnostic side: `crates/codex1/src/cli/status/project.rs:206-209` emits `"No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json."` — the PLAN.yaml is none of those in a deadlock case.

**Evidence:**

Create a mission where T4 is a review task with `depends_on: [T1,T2,T3]` and `review_target.tasks: [T1,T2,T3]`, and T2 depends on T1, T3 on T2 (the pattern a reviewer-minded skill might naturally produce):

```
$ codex1 plan check --json --mission deadlock --repo-root <root>
{ "ok": true, "revision": 4, "data": { "tasks": 4, "review_tasks": 1, "locked": true } }

$ codex1 task start T1 ... && codex1 task finish T1 --proof specs/T1/PROOF.md ...
# T1 → status awaiting_review (T4 targets it), per cli/task/finish.rs:56-61

$ codex1 task next --json ...
{ "data": { "next": { "kind": "blocked",
  "reason": "tasks awaiting review without a ready review task: T1" } } }

$ codex1 status --json ...
{ "data": { "next_action": { "kind": "blocked",
  "reason": "No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json." } } }
```

Two issues in one shape: (a) `plan check` locked a plan that cannot progress — T2 is blocked because T1 is in `awaiting_review` (non-review readiness, per `crates/codex1/src/state/readiness.rs:172-192` and `crates/codex1/src/cli/task/lifecycle.rs:144-156`, requires Complete/Superseded), and T4 cannot start because it depends on T2 and T3. (b) When the user hits the deadlock, `status.next_action.reason` suggests the PLAN is missing/empty/inconsistent, which it is not.

**Suggested fix:**

(a) In `cli/plan/check.rs::validate_tasks`, add a reachability pass after the DAG topo sort: for each non-review task, if any `depends_on` dep is a non-review task that appears in some review's `review_target.tasks` AND that review also depends on this task (transitive OK), reject with `PLAN_INVALID` + code/hint "review-loop deadlock: task `{tid}` depends on `{dep}` which will go `awaiting_review` and can only be reviewed via `{review_id}`, but `{review_id}` also depends on `{tid}`". (b) In `cli/status/project.rs::derive_next_action` around line 207, branch on actual state: if any task is `awaiting_review` with no ready review target, emit a different `reason` ("deadlock: T… awaiting review; review task T… blocked on …"); reserve the current message for the literal "plan file missing/empty" case.

### P2-2 — `status.ready_tasks` reports task IDs while `plan_locked:false`, verdict `needs_user`

**Citation:**
- Contract: `02-cli-contract.md:175` ("`phase, loop, next_action, close_ready, stop` must be internally consistent with verdict") — `ready_tasks` is not in the consistency set, but `02-cli-contract.md:193-194` shows `ready_tasks` in the canonical envelope and `01-product-flow.md:30-48` implies plan must be valid before ready_tasks mean anything.
- Code: `crates/codex1/src/cli/status/project.rs:34-37` builds `ready_tasks` purely from the derived `wave`, and the wave derivation in `crates/codex1/src/cli/status/next_action.rs:72-98` ignores `state.plan.locked` entirely. Compare `crates/codex1/src/cli/plan/waves.rs:66-79` which short-circuits when the plan isn't locked.

**Evidence:**

```
$ codex1 replan record --supersedes T1 --reason six_dirty --json ...
$ codex1 status --json ...
{ "data": {
    "plan_locked": false,
    "verdict": "needs_user",
    "next_action": { "kind":"plan", "hint":"Draft and lock PLAN.yaml." },
    "ready_tasks": ["T2"],
    "review_required": [{ "task_id":"T2", "targets":["T1"] }]
} }
```

`ready_tasks` advertises work the caller must not run (see P1-1 for why it's reachable). A skill reading `ready_tasks` nonempty + `next_action.kind:"plan"` gets mixed signals.

**Suggested fix:**

In `cli/status/project.rs::build`, short-circuit wave/review derivation when `!state.plan.locked` (emit `ready_tasks: []`, `review_required: []`, `parallel_safe: false`). Keep `next_action:{kind:"plan"}` as-is.

## P3 (non-blocking)

### P3-1 — `close.reviewers` for the mission-close review not persisted in `STATE.json`, so `CLOSEOUT.md` shows "—"

Code: `crates/codex1/src/cli/close/record_review.rs:113-122` (clean-path mutation only sets `review_state`; reviewers are only in the event payload). Renderer: `crates/codex1/src/cli/close/closeout.rs:87` prints a literal `—`. Running `close record-review --clean --reviewers c1` then inspecting `CLOSEOUT.md` yields `| MC | clean | — | — |` even though the reviewer names were passed. Suggested fix: extend `state.close` with optional `reviewers: Vec<String>` (and `findings_file: Option<String>`) and read them in `closeout.rs`.

### P3-2 — `status.close_ready:false` at `verdict:terminal_complete` is cosmetically confusing

Code: `crates/codex1/src/state/readiness.rs:70-72` makes `close_ready` strictly `== MissionCloseReviewPassed`. After close completes, verdict → `terminal_complete`, so `close_ready:false`. Defensible semantics ("no, close is not 'ready' — it's done") but worth a clarifying comment in the handoff or a rename to `close_complete_allowed` to avoid confusion. The field already agrees with `close check --json` (`ready:false` at terminal), so this is a polish item rather than a bug.
