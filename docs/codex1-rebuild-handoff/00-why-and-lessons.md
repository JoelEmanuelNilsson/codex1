# 00 Why And Lessons

This file explains why Codex1 exists and what the rebuild must learn from the earlier attempts.

Read this before designing code.

## Why Codex1 Exists

Codex is already strong at reasoning, editing code, running commands, and using subagents.

Codex1 should not replace Codex. Codex1 should give Codex better rails for long-running work.

The intended rails are:

```text
clear outcome
excellent plan
task DAG
derived waves
bounded work
planned review
repair/replan loop
mission close
tiny stop guard
```

The goal is not to build a second agent runtime.

The goal is to make native Codex sessions more reliable when the mission is larger than one turn.

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

The lesson is not "remove contracts."

The lesson is:

```text
Keep the contracts centered, simple, visible, and command-shaped.
Do not spread them across many hidden surfaces.
Do not invent fake role-security systems.
```

## What To Preserve

Preserve:

- Skills-first UX.
- A deterministic CLI substrate.
- Visible mission files.
- Outcome truth before planning.
- Plans with explicit task DAGs.
- Derived waves.
- Proof before review.
- Review before completion.
- Repair/replan from findings.
- `$close` as user discussion boundary.
- Ralph as a stop guard.
- Mission-close review as terminal gate.

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
```

Bad:

```text
CLI tries to know whether the caller is a reviewer.
Ralph reconstructs mission truth.
Reviewers write gate state.
Session IDs become product logic.
Several files all claim to own the same truth.
```

## Completion Principle

Work is complete only when all of these agree:

- The clarified outcome.
- The plan.
- Task proof.
- Planned review.
- Mission-close review.
- Close check.
- Final closeout.

Execution finishing is not enough.

Review being mostly clean is not enough.

Mission completion means the mission contract, proof, review, and closeout all agree.

## Discussion Mode Principle

If the user is talking to Codex, that is not automatically a request to continue execution.

`$close` should make discussion mode explicit:

```text
pause loop
allow stop
talk with user
resume or deactivate when decided
```

Ralph should not fight human communication.

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

Use distinct states:

- `ready_for_mission_close_review`
- `mission_close_review_open`
- `mission_close_review_passed`
- `terminal_complete`

Do not call a mission complete just because all normal tasks are done.

Do not call a mission terminal until mission-close review has passed and close check has completed.

