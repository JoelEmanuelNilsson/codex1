---
name: execute
description: >
  Run the next ready task or ready wave for the active Codex1 mission. Use after `$plan` has locked PLAN.yaml and before mission close. Reads `codex1 task next`, decides whether to run serially or spawn workers for a wave, collects proofs, and calls `codex1 task finish`. Hands to $review-loop when the next task kind is `review`. Respects parallel_safe. Does not record review findings or close the mission.
---

# Execute

## Overview

`$execute` runs one step at a time against the locked DAG: a single ready task, or one ready wave. It dispatches work (serially on the main thread or via worker subagents), writes per-task proofs, and calls `codex1 task finish`. It does not plan, replan, review, or close. When the next action is a review, `$execute` returns control to the main thread with guidance to invoke `$review-loop`.

## Preconditions

- A ratified `OUTCOME.md` exists.
- `PLAN.yaml` is locked. `codex1 --json status` must show `plan_locked: true` and `verdict: continue_required`.
- The loop is active (`loop.active: true`). If paused, resume via `$close` discussion flow first.
- Working directory is at repo root, or `--mission <id>` / `--repo-root <path>` resolve the mission.

If any precondition fails, stop and surface the missing condition to the user; do not attempt work.

## Required Workflow

Run the loop below until `task next` returns a kind outside `run_task` / `run_wave`, or until the loop is paused by `$close`.

### Step 1. Read the next action

```bash
codex1 --json status
```

`codex1 --json status` is the single source of truth (per `docs/codex1-rebuild-handoff/01-product-flow.md:151`); `task next` narrows to work-only kinds and cannot surface `repair` or `replan`. Inspect `data.next_action.kind`:

- `run_task` — run this single task. The main thread may run it serially (for small/local edits) or spawn one worker.
- `run_wave` with `parallel_safe: true` — spawn one worker per task in `tasks`.
- `run_wave` with `parallel_safe: false` — run the wave's tasks sequentially (one at a time), respecting `blockers` / `exclusive_resources`.
- `run_review` — return to the main thread with: "Next action is a review task. Hand to `$review-loop`." Do not record findings here.
- `mission_close_review` — return with: "Hand to `$review-loop` in mission-close mode."
- `repair` — the status payload lists the target ids under `data.next_action.task_ids`. Run the named repair task (same flow as `run_task`), then return to Step 1.
- `replan` — return with: "Replan required. Hand to `$plan replan`."

`$execute` returns control to the main thread on `run_review`, `mission_close_review`, and `replan`. It does not call into other skills.

### Step 2. Run a task

For each task id being executed:

```bash
codex1 --json task start T<id>
codex1 --json task packet T<id>
```

Build the worker prompt from `references/worker-packet-template.md` using fields from the packet envelope (`task_id`, `title`, `spec_excerpt`, `write_paths`, `proof_commands`, `mission_summary`). Spawn one worker subagent per task. See Worker Model Defaults below for model selection.

Main-thread serial alternative: if the task is small and the main thread has capacity, paste the packet into the main thread's own working context, edit inside `write_paths`, and run the listed `proof_commands` directly. Apply the same "you must not" boundaries as the worker template.

### Step 3. Record proof

For each finished task, write `specs/T<id>/PROOF.md` containing:

- The exact proof commands that were run (from `proof_commands`).
- Each command's actual output or a pointer to captured logs.
- Any blockers or assumptions the worker reported.
- A short list of changed files, if not already captured by the worker.

Proof is the artifact `codex1 task finish` will read; do not skip it.

### Step 4. Finish the task

```bash
codex1 --json task finish T<id> --proof specs/T<id>/PROOF.md
```

If the CLI returns `PROOF_MISSING`, fix the proof file and retry. If it returns `REVIEW_FINDINGS_BLOCK` or `TASK_NOT_READY`, stop the loop and surface the error — do not bypass it.

### Step 5. Loop

Return to Step 1. Continue until `task next` reports a kind other than `run_task` / `run_wave` / `repair`, or the loop is paused.

## Worker Model Defaults

Follow the matrix in `docs/codex1-rebuild-handoff/04-roles-models-prompts.md`:

- Coding worker (code-heavy): `gpt-5.3-codex` at reasoning `high`. Claude-family equivalent: `claude-opus-4-7` for code-heavy work.
- Intent-heavy worker (product judgment dominates coding): `gpt-5.4` at reasoning `high`. Claude-family equivalent: `claude-sonnet-4-6` (a.k.a. gpt-5.4).
- Small mechanical worker / spark-level edits: `gpt-5.3-codex-spark` at reasoning `high`. A haiku-class model is acceptable here.

Escalate a coding worker to `gpt-5.4` when product intent dominates the coding decisions. Do not use mini/haiku-class models for non-trivial edits. When unsure, default to the code-heavy coding worker.

## Worker Standing Instructions

Worker permissions, prohibitions, and the required report shape live in `references/worker-packet-template.md`. Always build worker prompts from that template — do not re-write the rules inline in a prompt, and do not widen them to grant access beyond the task's `write_paths` and `proof_commands`.

## Parallel Safety

When `run_wave` has `parallel_safe: false`:

- Run the wave's tasks one at a time in the order returned.
- Read `blockers` and, if present, `exclusive_resources` or `unknown_side_effects` in the status/wave payload — these tell you why the wave is not parallel-safe. Honor them (do not launch overlapping workers on the same exclusive resource).
- After each task finishes, re-read `codex1 --json task next` before starting the next one; the next action may have shifted (for example, to a review task).

When `parallel_safe: true`, the default is one worker per task in the wave. The main thread may still choose to run serially if worker cost outweighs the parallelism benefit for small tasks.

## Failure Modes

- Task has active findings from a prior review: `task next` returns `kind: repair`. Run the named repair task via Steps 2-4, then resume.
- Six consecutive dirty reviews on the same active target: `task next` returns `kind: replan`. Return to the main thread with guidance to invoke `$plan replan`; do not attempt to fix this inside `$execute`.
- Worker reports a blocker (missing dependency, ambiguous spec, external access, dangerous change): pause the loop, escalate to the user, or hand back so `$plan replan` can add or adjust tasks. Do not silently work around blockers.
- CLI returns `REVISION_CONFLICT`: re-read `codex1 --json status` to pick up the latest revision and retry the mutating command with the fresh `revision` in `--expect-revision`.
- CLI returns `REPLAN_REQUIRED` or `REVIEW_FINDINGS_BLOCK`: stop and surface — these are terminal for the current `$execute` invocation.

## Do Not

- Do not record review findings — that belongs to `$review-loop`.
- Do not write or mutate `STATE.json`, `EVENTS.jsonl`, `PLAN.yaml`, or `OUTCOME.md` directly; only the CLI mutates them.
- Do not mark the mission complete — that is `$close` plus `codex1 close complete`.
- Do not spawn reviewers, run `codex1 review record`, or write to `reviews/`.
- Do not advance past `task next` when it reports `run_review`, `mission_close_review`, or `replan`.

## References

- `references/worker-packet-template.md` — the exact worker prompt template built from `codex1 task packet` output. Load it when building a worker spawn prompt.
