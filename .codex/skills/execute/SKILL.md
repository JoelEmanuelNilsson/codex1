---
name: execute
description: Ralph-governed execution workflow for Codex1. Use when the user invokes $execute and the mission already has a passed execution package for the selected mission, spec, or wave target.
---

# Execute

Use this public skill to advance one execution-safe target at a time.

## Entry Rule

Do not start from blueprint approval alone.

`$execute` only runs a target that already has a passed execution package.
If the next frontier is not currently packaged, route back to planning or
execution-package preparation instead of improvising.

Deterministic backend:

- begin a parent execution loop with `codex1 internal begin-loop-lease` using
  `mode = "execution_loop"` before relying on Ralph continuation
- validate the target with `codex1 internal validate-execution-package`
- derive bounded child write briefs with `codex1 internal derive-writer-packet`
- when execution discovers a non-local contract break, write a structured
  record with `codex1 internal record-contradiction`
- end every bounded cycle with `codex1 internal append-closeout` when a more
  specific helper did not already do it; that low-level helper is only for
  non-terminal Ralph verdicts, and terminal `complete` / `hard_blocked`
  outcomes still require a machine-checkable terminal closeout path rather than
  prose-only finish language

## Ralph Lease

`$execute` is a parent/orchestrator loop. When the user invokes `$execute`,
acquire or refresh a parent lease before autonomous continuation:

```json
{
  "mission_id": "<mission-id>",
  "mode": "execution_loop",
  "owner": "parent-execute",
  "reason": "User invoked $execute."
}
```

Use `codex1 internal begin-loop-lease` for that payload. If the user interrupts
to talk, use the close/pause surface instead of continuing to force execution.

## Execution Posture

- Execute advances one execution-safe target at a time.
- Package truth outranks conversational momentum.
- `mission:<id>` means "advance the current executable frontier for this
  mission," not "finish the whole mission in one jump."
- Review, repair, replan, and durable waiting are part of execution truth, not
  side exits from it.

## Target Resolution

Accept only targets that resolve unambiguously to one of:

- `mission:<id>`
- `spec:<id>`
- `wave:<id>`
- a clean path that resolves to exactly one of the above

`mission:<id>` means "run the current next selected executable target for this
mission," not "run the whole mission at once."

## Workflow

1. Resolve the exact target against current mission truth and validate that its
   execution package is still passed and fresh.
2. If the target resolves to `mission:<id>`, bind yourself to the current
   frontier slice the package actually authorizes.
3. If you will delegate write-capable work, derive a bounded writer packet from
   that package first.
4. Execute only inside the declared read and write scope.
5. Gather proof receipts as you go and keep the spec support files honest:
   - `SPEC.md`
   - `REVIEW.md`
   - `NOTES.md`
   - `RECEIPTS/`
6. Default to one writer at a time in the current workspace. Parallel write
   work is allowed only when graph safety is explicit, `write_paths` are
   pairwise disjoint, no same-wave `write_paths` overlap another task's
   `read_paths`, exclusive resources are disjoint, and no shared
   schema/deploy/lockfile/global-config side effects remain. Unknown side
   effects default back to singleton execution.
7. If the target reaches a blocking review gate, route into `$review-loop`.
8. If repair stays inside the current contract, perform bounded repair and keep
   the package truth current.
9. If execution crosses the declared replan boundary or breaks a broader
   contract, invoke `internal-replan` and continue only from reopened truth.
10. If the current frontier is clean and the remaining owed gate is
   mission-close review, route into `$review-loop` for the mission-close bundle
   instead of declaring completion from execution.
11. If the honest branch is `needs_user`, leave durable waiting state and stop
   only as a non-terminal yield.
12. End every bounded execution cycle with explicit durable state, not
   conversational implication.

## Execution Rules

- One runnable workstream spec is one execution graph node.
- Do not expand scope because the current target feels nearby.
- Treat review obligations and proof obligations as first-class gate inputs, not
  cleanup.
- If a proof row is blank, the spec is not complete.
- If the source package is stale, superseded, failed, or consumed without fresh
  revalidation, execution is not authorized.
- If a branch is blocked by review, repair, or replan, do not pretend execution
  can keep writing through it.
- Non-terminal waiting is not terminal completion.
- `wait_agent` or child completion signals do not by themselves prove parent
  integration or mission completion.
- A clean final frontier still owes mission-close review before completion.

## Must Not

- start from a spec that lacks a passed package gate
- exceed declared write scope
- choose broader architecture inside a writer lane
- treat review as optional or post-hoc polish
- continue past a blueprint or mission-lock contradiction without reopening
- declare the mission done from execution alone when mission-close review is
  still owed

## Return Shape

Each execution cycle should leave an honest verdict plus refreshed proof,
receipt, and review-preparation state for the selected target.

Only `complete` and `hard_blocked` are terminal. `needs_user` is a durable
waiting non-terminal, and post-review / post-repair / post-replan branches must
continue inside the same Ralph-governed execution flow.
