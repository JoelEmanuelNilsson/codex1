# Round 2 — e2e-walkthrough audit

## Summary

Drove a full mission happy path through `~/.local/bin/codex1` (built from HEAD `05fcae3` via `make install-local`) in `/tmp/codex1-review-e2e-r2/`: 4-task DAG (3 work + 1 review), ratify → choose-level → scaffold → plan check → loop activate → T1/T2/T3 start+finish → review T4 → mission-close review → close complete → terminal. Final STATE.revision=15, EVENTS.jsonl=15 strictly monotonic seqs, CLOSEOUT.md written, `stop.allow` transitions correct (`idle` → `active_loop` → `terminal`), Ralph stop hook exits 2 on active loop and 0 on terminal.

All round-2 regression checks hold:
- `choose-level --level hard --escalate "reason"` → `escalation_required: false`, no `escalation_reason` field.
- `choose-level --level light --escalate "reason"` → `escalation_required: true`, `escalation_reason: "..."`, `effective_level: hard` (auto-bumped).
- After `replan record`: `status.ready_tasks: []`, `review_required: []`, `parallel_safe: false`.
- After `replan record`: `task start`, `task finish`, `review start`, `review record` all return `PLAN_INVALID` with message "plan is not locked; cannot mutate tasks or reviews until it is" and hint "Run `codex1 plan check` to lock PLAN.yaml first." Fix at `crates/codex1/src/state/mod.rs:63-71` is wired into all four sites.
- Deadlock plan from round-1 (review targets and deps overlap, work tasks cycle through review) is rejected by `plan check` with `PLAN_INVALID: review-loop deadlock: task `T2` depends on `T1` which can only be reviewed by `T4`, but `T4` transitively depends on `T2`` — message mentions "deadlock" explicitly.

Failure-path error codes confirmed: `OUTCOME_INCOMPLETE`, `PLAN_INVALID` (for task/review mutations on unlocked plan), `TASK_NOT_READY` (for start with incomplete deps), `PROOF_MISSING`, `REPLAN_REQUIRED` (after 6× dirty), `CLOSE_NOT_READY`, `TERMINAL_ALREADY_COMPLETE`. No additional deadlock shapes surfaced for realistic skill outputs.

Two new findings surfaced, neither a regression on round-1's scoped fixes: **one P0** (state.replan.triggered is never cleared; a mission that enters replan is bricked and cannot close), **one P2** (`task next` does not gate on `plan.locked` or `replan.triggered`, contradicting `status.next_action`/`ready_tasks`).

## P0

### P0-1 — `state.replan.triggered` is never cleared, bricking any mission that enters replan

**Citation:**
- Handoff `docs/codex1-rebuild-handoff/02-cli-contract.md:175` — "`phase, loop, next_action, close_ready, stop` must be internally consistent with verdict"; L208 — "`codex1 status` and `codex1 close check` must share readiness logic. They must not disagree about whether a mission is complete."
- Handoff `docs/codex1-rebuild-handoff/05-build-prompt.md:87` — "six consecutive dirty reviews trigger replan; clean resets the consecutive count" (implies that once the replan cycle completes, the trigger state resolves).
- Handoff `docs/codex1-rebuild-handoff/01-product-flow.md:42-46` — replan returns to the Waves execution flow, implying replan is a completable operation.
- Diverging code: `grep -rn "\.replan\.triggered\s*=" crates/codex1/src` returns 7 write sites, all of which set the flag to `true`: `crates/codex1/src/cli/replan/record.rs:151`, `crates/codex1/src/cli/review/record.rs:347`, `crates/codex1/src/cli/close/record_review.rs:200`, `crates/codex1/src/state/readiness.rs:155, 177`. Zero write sites set it to `false`. `crates/codex1/src/cli/plan/check.rs` does not mutate the field (grep `replan\.triggered` in that file returns no matches).
- The existing integration test `crates/codex1/tests/e2e_replan_trigger.rs:237` asserts `state["replan"]["triggered"] == true` right after `replan record` and stops — it never verifies the post-relock path clears the flag, which is where the gap bites.

**Evidence (reproducer):**

```
# Standard 4-task plan (T1,T2,T3 work; T4 review of T2,T3). Start T1 to register
# it in STATE.tasks, then trigger a replan:
$ codex1 task start T1 --mission demo --json
$ codex1 replan record --reason scope_change --supersedes T1 --json --mission demo
# ok:true, plan_locked:false, phase_after:"plan", STATE.replan.triggered:true

# Author appends a new task in PLAN.yaml (or re-uses the existing one in the
# reproducer) and relocks:
$ codex1 plan check --mission demo --json
# ok:true, locked:true   -- but replan.triggered stays true in STATE.json

$ codex1 status --mission demo --json
# data.replan_required: true
# data.next_action: {"kind":"replan","command":"$plan replan","reason":"scope_change"}
# data.verdict: blocked

# Complete the re-planned work and mission-close review cleanly:
$ codex1 task start T2 && codex1 task finish T2 --proof ...
$ codex1 task start T3 && codex1 task finish T3 --proof ...
$ codex1 review start T4 && codex1 review record T4 --clean --reviewers r1

$ codex1 close check --mission demo --json
# ok:true, ready:false, verdict:"blocked",
# blockers:[{"code":"REPLAN_REQUIRED","detail":"scope_change"},
#           {"code":"CLOSE_NOT_READY","detail":"mission-close review has not started"}]

$ codex1 close record-review --clean --reviewers c1 --mission demo --json
# ok:false, code:"CLOSE_NOT_READY",
# message:"cannot record mission-close review while verdict is `blocked` (REPLAN_REQUIRED: scope_change…)"

$ codex1 close complete --mission demo --json
# ok:false, code:"CLOSE_NOT_READY", message:"REPLAN_REQUIRED: scope_change; …"
```

At `cli/close/check.rs:87-93` the blocker list pulls directly from `state.replan.triggered` and `state.replan.triggered_reason`. At `cli/status/project.rs:70,151-156` the status projection does the same. Because no code path ever writes `triggered = false`, any mission that successfully replans can never close. The user's only "out" is to edit `STATE.json` directly — forbidden by the handoff (`05-build-prompt.md`: "Do not edit `STATE.json` or `EVENTS.jsonl` directly").

There is a parallel secondary inconsistency that falls out of the same root: after a `review record --clean` with a non-zero counter post-trigger, `state.replan.consecutive_dirty_by_target` is reset to `{}` (by `apply_clean` at `cli/review/record.rs:294-317`) but `triggered` stays `true`. `codex1 replan check` then returns `required: false, reason: null, triggered_already: true`, while `codex1 status` still returns `replan_required: true, next_action.kind: "replan"` — the "status and close check agree" invariant (`02-cli-contract.md:208`, `05-build-prompt.md:89`) is broken.

**Severity — P0 (product-breaking):** the mission cannot reach `terminal_complete` by any legitimate sequence of CLI calls after it enters replan. Autopilot (state-machine at `.codex/skills/autopilot/references/autopilot-state-machine.md:19`) dispatches on `verdict=blocked, kind=replan` → `$plan replan`, which would loop forever because status never clears.

**Suggested fix:** in `cli/plan/check.rs`, after a successful relock, clear the replan trigger alongside resetting any per-target dirty counters: `state.replan.triggered = false; state.replan.triggered_reason = None;`. Alternatively, move the gate into `cli/replan/record.rs` such that completing the replan record (with at least one `--supersedes` or with a new `plan_hash` computed at relock) flips the flag. The existing integration test `e2e_replan_trigger.rs::e2e_six_dirty_reviews_trigger_replan_and_record_clears_counters` should be extended past line 252 to run `plan check` and assert `state["replan"]["triggered"] == false`.

## P1

None.

## P2

### P2-1 — `codex1 task next` ignores `plan.locked` and `replan.triggered`; contradicts `status` after `replan record` and when a replan is pending

**Citation:**
- Handoff `docs/codex1-rebuild-handoff/02-cli-contract.md:363-365` — "`codex1 task next --json`: Returns the next ready task or wave." (If plan is unlocked or a replan is required, no task is "ready" by the rest of the contract.)
- Handoff `05-build-prompt.md:84` — "task next reports ready task/wave."
- Round-1 e2e P2-2 fix (decisions.md L22): `cli/status/project.rs::build` short-circuits `ready_tasks: [], review_required: [], parallel_safe: false` when `plan.locked` is false. The same short-circuit is missing from `task next`.
- Diverging code: `crates/codex1/src/cli/task/next.rs:16-60` reads plan + state and derives readiness without checking `state.plan.locked` or `state.replan.triggered`. Compare `crates/codex1/src/cli/status/project.rs:34-70` which was patched and `crates/codex1/src/cli/plan/waves.rs:66-79` which short-circuits on unlocked plans.

**Evidence:**

```
# Post replan record (plan.locked=false, replan.triggered=true):
$ codex1 --mission replan_test --json status
{ "data": { "plan_locked": false, "ready_tasks": [], "review_required": [],
           "next_action": {"kind":"plan","hint":"Draft and lock PLAN.yaml."} } }

$ codex1 --mission replan_test --json task next
{ "data": { "next": {"kind":"run_wave","wave_id":"W2","tasks":["T2","T3"],
                     "parallel_safe":true} } }

# Post 6× dirty (plan.locked=true, replan.triggered=true):
$ codex1 --mission dirty6 --json status
{ "data": { "replan_required": true,
           "next_action": {"kind":"replan","command":"$plan replan",
                           "reason":"6 consecutive dirty reviews for T3"} } }

$ codex1 --mission dirty6 --json task next
{ "data": { "next": {"kind":"run_review","task_id":"T4",
                     "targets":["T2","T3"]} } }
```

Round-1 patched `status` to report empty ready/review lists when the plan is unlocked, but a skill that calls `task next` directly (as suggested by `docs/codex1-rebuild-handoff/01-product-flow.md:151` "codex1 status / task next") will receive `run_wave W2 [T2,T3]` and could attempt `task start T2`, which then correctly refuses with `PLAN_INVALID`. The mutation gate holds — no state corruption — but the advisory contract is broken: two canonical readiness endpoints disagree, and `task next`'s output is not actionable.

**Severity — P2 (correctness-adjacent, not P1):** `task start` still fails closed with `PLAN_INVALID`, so there is no silent state mutation. The damage is misguided skills: a naive worker that calls `task next` → `task start` will hit a stop-the-world error, and autopilot following `status.next_action.kind` may disagree with a sibling skill following `task next`.

**Suggested fix:** in `cli/task/next.rs::run`, insert the same two short-circuits the round-1 P2-2 fix applied to `status`:

```rust
if !state.plan.locked {
    return JsonOk::ok(..., json!({
        "next": {"kind":"plan","hint":"Draft and lock PLAN.yaml."}
    }));
}
if state.replan.triggered {
    return JsonOk::ok(..., json!({
        "next": {"kind":"replan","reason": state.replan.triggered_reason.clone().unwrap_or_default()}
    }));
}
```

Test: an integration test analogous to `tests/status.rs::unlocked_plan_emits_empty_ready_tasks_and_review_required` but calling `task next`, asserting `data.next.kind == "plan"` after `replan record`.

## P3

None that are in scope for the loop. Drive-by observations (non-blocking, not P0/P1/P2):

- Revision bumps are unit-sized and match seq in EVENTS.jsonl 1:1 across the full happy path (demo: 15 events, seq=1..15, STATE.revision=15).
- `plan graph --json` returns a mermaid flowchart with color classes; not exercised under load here.
- `codex1 status --json` with no `--mission` in a repo with multiple `PLANS/*/` dirs returns `{verdict:needs_user, stop:{allow:true, reason:"no_mission"}, foundation_only:true}` gracefully — Ralph stop hook exits 0 in that case as documented in the hook script.
