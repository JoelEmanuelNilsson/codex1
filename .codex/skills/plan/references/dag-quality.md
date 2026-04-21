# DAG Quality

Heuristics for designing the Codex1 task DAG inside `PLAN.yaml`. Read this when the DAG is more than a straight line, when parallel-safe waves matter, or when `codex1 plan check` keeps returning `PLAN_INVALID`.

## Root tasks

Root tasks have `depends_on: []`. Keep them independent. If two roots share files, one of them is not really a root; give it an explicit dependency or merge them.

- Prefer design/research tasks as roots when architecture is open.
- Prefer a single schema/contract root when multiple downstream coders need the same interface.
- Avoid more than three or four roots unless the mission is genuinely parallel at the source.

## Wave shape

Waves are derived by `codex1 plan waves` from `depends_on`. A wave includes every task whose dependencies are satisfied and whose state is ready. Design the DAG so each wave is deliberate.

- A wave should either be parallel-safe (no shared `write_paths`, no shared `exclusive_resources`, no overlapping `schema_or_migration`) or explicitly serial (tasks lined up by dependency).
- Two tasks in the same wave that both write to the same crate or the same module are usually a DAG bug, not parallelism. Split them serially or merge them.
- Use `exclusive_resources` to mark shared identities that are not files — a stored schema, a global lockfile, a migration id, a daemon port.

## Executable task fields

Every executable task should answer four questions:

1. What does it read? (`read_paths`)
2. What does it write? (`write_paths`)
3. What resources does it block? (`exclusive_resources`, `schema_or_migration`, `package_manager_mutation`, `env_mutations`)
4. How do we know it worked? (`acceptance`, `proof`)

Leaving any of these blank when they apply causes `plan check` failures, unsafe parallelism, or unreviewable work.

## Review task placement

Review tasks do not advance the work. They gate correctness. Place them:

- After a code wave that lands a user-facing capability.
- Before a wave that depends on unverified behavior from the previous wave.
- At every subsystem seam.
- After any task that is marked `unknown_side_effects: true`.
- Before mission close (always).

A review task's `depends_on` must include every task in `review_target.tasks`. Tasks downstream of a planned review should depend on the review task, not on the reviewed work directly, so review-clean gating works.

## Replans

When replanning, append new tasks with new IDs. Never reuse an ID. Use `codex1 replan record --reason <code> --supersedes <id>` for tasks being abandoned (see `crates/codex1/src/cli/replan/triggers.rs::ALLOWED_REASONS` for the reason set). The DAG can contain superseded tasks; the CLI excludes them from ready waves.

- Name the failure class in `planning_process.evidence` (advisor or plan_review entry).
- Make the new tasks smaller and more specific than the ones they replace; replans that look like the original plan repeat the original failures.

## Common failures and fixes

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `DAG_CYCLE` | A downstream task appeared in an upstream `depends_on`. | Trace the cycle via `plan graph --format mermaid`; remove the back-edge. |
| `DAG_MISSING_DEP` | `depends_on` lists a task id that does not exist. | Fix typo, or add the missing task. |
| Wave has conflicting writes | Two tasks share `write_paths` or `exclusive_resources`. | Serialize by adding a dependency, or merge tasks. |
| Reviewer cannot verify | Task lacks `proof` or `acceptance`. | Add explicit commands and testable criteria. |
| Replan triggers repeatedly | Review target is too broad. | Split the work into smaller tasks with tighter review boundaries. |

## Sanity check before lock

Run all three:

```bash
codex1 --json plan check
codex1 --json plan graph --format mermaid
codex1 --json plan waves
```

Read the mermaid graph visually and confirm:

- Every leaf of the DAG is either a review task or a mission-close-criteria contributor.
- Every review task sits on the critical path of the work it is reviewing.
- No wave is larger than the main thread can orchestrate comfortably.
