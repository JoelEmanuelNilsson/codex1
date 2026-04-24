# 00 Why And Lessons

This file explains why Codex1 exists and what the rebuild must learn from the earlier attempts.

Read this before designing code.

## Why Codex1 Exists

Codex is already strong at reasoning, editing code, running commands, and using subagents.

Codex1 should not replace Codex. Codex1 should give Codex better rails for work that has enough ambiguity, risk, duration, or coordination cost that ordinary chat memory becomes fragile.

The intended rails are:

```text
clear outcome
risk-scaled plan
bounded work
evidence loop
repair/replan loop
completion gate
minimal stop guard
```

For large and risky missions, the rails become stricter:

```text
ratified outcome
graph plan
explicit dependencies
derived waves
planned review tasks
mission-close review
terminal close
```

The goal is not to build a second agent runtime.

The goal is to make native Codex sessions more reliable when the mission is larger than one turn, while keeping small work light.

## What Went Wrong Before

The old implementation got many ideas right, but the control plane became too noisy.

Concrete failure modes from earlier runs:

- Too many truth surfaces.
- Gate/fingerprint interactions that made artifacts stale too easily.
- Review bundles becoming stale because receipts changed, then receipt changes changing package truth again.
- Stale reviewer agents continuing after their review boundary was superseded.
- Late reviewer outputs appearing after a gate had already been marked passed.
- Parent authority tokens getting lost across resumed turns.
- Manual lease-file recovery.
- Ralph blocking the wrong actor or the wrong moment.
- User discussion being treated like an instruction to continue the active loop.
- Mission-close language that sounded complete before mission-close review actually happened.
- Reviewers or child lanes appearing to mutate mission truth.
- Heavy DAG mechanics being treated as the default even when normal planning was enough.

The lesson is not "remove contracts."

The lesson is:

```text
Keep the contracts centered, simple, visible, and command-shaped.
Scale process to risk.
Do not spread truth across many hidden surfaces.
Do not invent fake role-security systems.
```

## What To Preserve

Preserve:

- Skills-first UX.
- A deterministic CLI substrate.
- Visible mission files when durable memory matters.
- Outcome truth before serious planning.
- Lightweight normal planning for ordinary multi-step work.
- Graph planning for large/risky/multi-agent work.
- Derived graph waves, not stored wave truth.
- Proof before claiming completion.
- Review before completion, scaled to risk.
- Repair/replan from findings.
- `$interrupt` as the user discussion boundary.
- Ralph as a status-only stop guard.
- Mission-close review as the terminal gate for graph/large/risky missions.

## What To Reject

Reject:

- Hidden daemons.
- Wrapper runtimes around Codex.
- `.ralph` as mission truth.
- Fake permission enforcement for subagent roles.
- Caller identity checks.
- Capability token mazes.
- Reviewer writeback authority systems.
- Stored waves as editable truth.
- Universal DAG planning for small and normal work.
- Public `$finish` or `$complete` skills.
- Many competing closeout/gate/cache files.
- CLI commands that require parsing giant prose.
- Autopilot as a separate hidden runtime.

## The Simplicity Principle

The answer is not fewer contracts.

The answer is better-centered contracts with less noisy operation.

Good:

```text
CLI validates artifacts and state transitions.
Skills guide Codex behavior.
Subagent prompts govern role behavior.
Ralph reads one status command.
Normal plans stay normal.
Graph plans carry the heavier machinery.
```

Bad:

```text
CLI tries to know whether the caller is a reviewer.
Ralph reconstructs mission truth.
Reviewers write gate state.
Session IDs become product logic.
Several files all claim to own the same truth.
Every request becomes a graph.
```

## Planning Principle

Planning is a control loop, not paperwork.

The plan should be just strong enough to keep intent stable, make the next action clear, and make repair possible.

Use two planning modes:

- Normal: lightweight build contract, acceptance criteria, validation, and soft status. It can be chat-only for small/local work or durable for ordinary multi-step work.
- Graph: explicit task graph, dependencies, derived waves, planned reviews, and stricter status.

Graph mode is for large, risky, cross-system, multi-agent, long-running, migration-like, security-sensitive, or hard-to-reverse work. It is not the default shape of thinking.

## Completion Principle

Work is complete only when the relevant evidence agrees.

For normal work:

- The requested outcome or checklist is satisfied.
- Relevant tests or checks passed, or failures are explained as unrelated.
- The diff was inspected for accidental scope.
- Any meaningful residual risk is known.

For graph/large/risky work:

- The clarified outcome is still matched.
- The graph plan is valid or explicitly revised.
- Required non-superseded tasks are complete or review-clean.
- Required proof exists.
- Planned review tasks are clean.
- Mission-close review has passed.
- `codex1 close check` and `codex1 status` agree.
- Terminal close has been recorded.

Execution finishing is not enough.

Review being mostly clean is not enough.

Mission completion means the mission contract, proof, review, and closeout all agree.

## Discussion Mode Principle

If the user is talking to Codex, that is not automatically a request to continue execution.

`$interrupt` should make discussion mode explicit:

```text
pause loop
allow stop
talk with user
resume or deactivate when decided
```

Ralph should not fight human communication.

## Ralph Principle

Ralph should be boring.

It reads `codex1 status --json` and makes one stop decision.

It should not inspect plan files, review files, task files, chat history,
subagent identity, or hidden hook state. The one Codex hook field Ralph uses
directly is the official `stop_hook_active` circuit breaker: when it is true,
Ralph allows stop.

It should also not be brittle:

- No active mission: allow stop.
- Normal mode: fail open unless status clearly says there is an active, unpaused, safe autonomous next action.
- Graph mode: be stricter while the active loop is valid, unpaused, and has an autonomous next action.
- Corrupt/invalid status should warn and allow stop rather than trapping the user in a broken loop.
- Ralph should block at most once per Stop-hook continuation cycle.

CLI mutations can fail closed. Ralph stop decisions should avoid wedging the user.

## Late Output Principle

Older agents may respond after the mission has moved on.

The system must have vocabulary for that:

- `accepted_current`: output was recorded before the active boundary closed.
- `late_same_boundary`: output arrived late but still applies to the same active boundary.
- `stale_superseded`: output belongs to a superseded task/review boundary.
- `contaminated_after_terminal`: output arrived after terminal completion.

Only current accepted outputs affect current truth.

Late/stale outputs may be logged for audit, but must not silently change mission state.

## Mission-Close Vocabulary

Use distinct close substates:

- `ready_for_mission_close_review`
- `mission_close_review_open`
- `mission_close_review_passed`
- `close_complete_ready`

These are close substates, not top-level terminal truth. Terminal completion
lives in `STATE.json.terminal.complete`. The canonical post-lock status verdicts
are in `08-state-status-and-graph-contract.md`.

Do not call a graph/large/risky mission complete just because all normal tasks are done.

Do not call a graph/large/risky mission terminal until mission-close review has passed and close check has completed.
