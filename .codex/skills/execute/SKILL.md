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

## Binary resolver

Every skill starts by resolving the V2 `codex1` binary to `$CODEX1`.

```bash
CODEX1="$("${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/resolve-codex1-bin")" || {
  echo "V2 codex1 not found. Set CODEX1_REPO_ROOT=<codex1 checkout> or build with: cargo build -p codex1 --release" >&2
  exit 1
}
```

Use `"$CODEX1"` for every `codex1` invocation below.

## Steps

1. Activate the execute parent loop (Ralph will now block stop):
   ```bash
   "$CODEX1" parent-loop activate --mission <id> --mode execute --json
   ```

2. Ask the CLI what to do next:
   ```bash
   "$CODEX1" task next --mission <id> --json
   ```
   Expect `next_action.kind: start_task` with a `task_id`.

3. Inspect the whole eligible wave. Status also returns:
   - `ready_tasks: [...]` — every eligible task id in the current wave.
   - `ready_wave_parallel_safe: bool` — true when the wave has ≥ 2 tasks
     and every wave-safety check passes (disjoint writes, no read/write
     conflicts, disjoint exclusive resources, no unknown side effects,
     no intra-wave dep edges).

   **If `ready_wave_parallel_safe` is `true`, start every id in `ready_tasks`
   before finishing any of them.** Review bundles still serialize per task.

   Otherwise, work only on `next_action.task_id` (the first id) and
   re-query status after each finish.

4. Start each task you have elected to run:
   ```bash
   "$CODEX1" task start --mission <id> <task_id> --json
   ```
   Capture the returned `task_run_id` — reviewers bind to it.

5. Read the spec at `specs/<task_id>/SPEC.md`, implement changes, and
   write `specs/<task_id>/PROOF.md` with the receipts the spec demands.

6. Finish the task — the CLI hashes the proof file:
   ```bash
   "$CODEX1" task finish --mission <id> <task_id> --json
   ```

7. Status will now emit `next_action.kind: review_open`. Hand over to
   `$review-loop` unless the user paused via `$close`.

8. When every task is terminal (`review_clean` or `complete` — or
   `superseded`), hand off to `$close`. **Do not run
   `parent-loop deactivate` here.** Terminalizing the mission still
   requires the mission-close review bundle and `mission-close
   complete`; deactivating the loop before that point drops Ralph
   pressure at exactly the frontier where mission-close needs to
   fire. `$close` owns the deactivate step — after a clean
   `mission-close complete`, it issues `parent-loop deactivate` as the
   terminal transition.

## Stop boundaries

- Once activated, Ralph blocks stop (`allow_stop: false`) until pause or
  deactivate. Use `$close` to pause for discussion.
- `$execute` never self-reviews. It hands off to `$review-loop`.
- `$execute` never replans. If `codex1 replan check` reports a mandatory
  trigger, return to `$plan`.
- `$execute` never deactivates the parent loop. Deactivate belongs to
  `$close` after mission-close-complete (Round 13 P1 fix).

## On stale proof

If the user edited a proof file after `task finish`, the stored
`proof_hash` diverges. `$review-loop` will detect this via `STALE_OUTPUT`;
if that happens, re-run `task start → finish` to refresh.
