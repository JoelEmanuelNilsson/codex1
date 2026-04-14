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

- validate the target with `codex1 internal validate-execution-package`
- derive bounded child write briefs with `codex1 internal derive-writer-packet`
- when execution discovers a non-local contract break, write a structured
  record with `codex1 internal record-contradiction`
- end every bounded cycle with `codex1 internal append-closeout` when a more
  specific helper did not already do it; that low-level helper is only for
  non-terminal Ralph verdicts

## Target Resolution

Accept only targets that resolve unambiguously to one of:

- `mission:<id>`
- `spec:<id>`
- `wave:<id>`
- a clean path that resolves to exactly one of the above

`mission:<id>` means "run the current next selected executable target for this
mission," not "run the whole mission at once."

## Workflow

1. Verify that the selected target has a passed execution package.
2. If you will delegate write-capable work, derive a bounded writer packet from
   that package first.
3. Execute only inside the declared read and write scope.
4. Gather proof receipts as you go and keep the spec support files honest:
   - `SPEC.md`
   - `REVIEW.md`
   - `NOTES.md`
   - `RECEIPTS/`
5. Default to one writer at a time in the current workspace. Parallel write
   work is allowed only when graph safety and path disjointness are explicit.
6. If the target reaches a blocking review gate, route into `$review`.
7. If repair stays inside the current contract, perform bounded repair and keep
   the package truth current.
8. If execution crosses the declared replan boundary or breaks a broader
   contract, invoke `internal-replan` and continue only from reopened truth.
9. End every bounded execution cycle with explicit durable state, not
   conversational implication.

## Execution Rules

- One runnable workstream spec is one execution graph node.
- Do not expand scope because the current target feels nearby.
- Treat review obligations and proof obligations as first-class gate inputs, not
  cleanup.
- If a proof row is blank, the spec is not complete.
- If the source package is stale, superseded, failed, or consumed without fresh
  revalidation, execution is not authorized.

## Must Not

- start from a spec that lacks a passed package gate
- exceed declared write scope
- choose broader architecture inside a writer lane
- treat review as optional or post-hoc polish
- continue past a blueprint or mission-lock contradiction without reopening

## Return Shape

Each execution cycle should leave an honest verdict plus refreshed proof,
receipt, and review-preparation state for the selected target.
