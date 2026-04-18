---
name: execute
description: Codex1 V2 execution. Use when the user invokes $execute, when plan waves has eligible tasks, or when the parent loop is in execute mode and a task is ready.
---

# $execute (Codex1 V2)

Advance the mission by running the next eligible task: activate the parent
loop, start the task, implement + write proof, finish the task, and hand to
`$review-loop` once proof is submitted.

## When to use

- The user invokes `$execute`.
- `codex1 task next` returns `next_action.kind: start_task`.
- The parent loop is in `execute` mode (re-entry after pause).

## Steps

1. Activate the execute parent loop (Ralph will now block stop):
   ```bash
   codex1 parent-loop activate --mission <id> --mode execute --json
   ```

2. Ask the CLI what to do next:
   ```bash
   codex1 task next --mission <id> --json
   ```
   Expect `next_action.kind: start_task` with a `task_id`.

3. Start the task:
   ```bash
   codex1 task start --mission <id> <task_id> --json
   ```
   Capture the returned `task_run_id` — reviewers bind to it.

4. Read the spec at `specs/<task_id>/SPEC.md`, implement changes, and
   write `specs/<task_id>/PROOF.md` with the receipts the spec demands.

5. Finish the task — the CLI hashes the proof file:
   ```bash
   codex1 task finish --mission <id> <task_id> --json
   ```

6. Status will now emit `next_action.kind: review_open`. Hand over to
   `$review-loop` unless the user paused via `$close`.

7. If there are no more ready tasks and none owe review, deactivate:
   ```bash
   codex1 parent-loop deactivate --mission <id> --json
   ```

## Stop boundaries

- Once activated, Ralph blocks stop (`allow_stop: false`) until pause or
  deactivate. Use `$close` to pause for discussion.
- `$execute` never self-reviews. It hands off to `$review-loop`.
- `$execute` never replans. If `codex1 replan check` reports a mandatory
  trigger, return to `$plan`.

## On stale proof

If the user edited a proof file after `task finish`, the stored
`proof_hash` diverges. `$review-loop` will detect this via `STALE_OUTPUT`;
if that happens, re-run `task start → finish` to refresh.
