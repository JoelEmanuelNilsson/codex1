---
name: plan
description: Codex1 V2 planning. Use when the user invokes $plan, when OUTCOME-LOCK is ratified but no DAG exists, or when replanning adds tasks.
---

# $plan (Codex1 V2)

Author the mission DAG inside `PROGRAM-BLUEPRINT.md` between the
`<!-- codex1:plan-dag:start -->` / `:end -->` markers. A plan without a DAG
is narrative-only and not executable.

## When to use

- The user invokes `$plan`.
- `OUTCOME-LOCK.md` is `ratified` and `plan waves` returns `waves: []`.
- A replan needs new task IDs appended (never reused; may supersede).

## Steps

1. Read the lock and any existing blueprint:
   ```bash
   codex1 validate --mission <id> --json
   codex1 plan graph --mission <id> --json    # current DAG, if any
   ```

2. Author task rows. Required fields per task:
   `id` (`^T[0-9]+$`), `title`, `kind`, `depends_on`, `spec_ref`,
   `read_paths`, `write_paths`, `exclusive_resources`, `proof`,
   `review_profiles`. Optional: `unknown_side_effects`, side-effect
   declarations (`generated_paths`, `shared_state`, `commands`,
   `external_services`, `env_mutations`, `package_manager_mutation`,
   `schema_or_migration`), `supersedes`.

3. For each task, write `specs/T<N>/SPEC.md` so the proof + review layers
   have something to bind against.

4. Transition each task to `ready` in `STATE.json` once its spec exists
   and deps are satisfied.

5. Verify:
   ```bash
   codex1 plan check  --mission <id> --json
   codex1 plan waves  --mission <id> --json
   codex1 task next   --mission <id> --json
   ```

## Stop boundaries

- `$plan` does **not** start tasks — that is `$execute`'s job.
- `$plan` does **not** run reviewers.
- Task IDs already written to `PROGRAM-BLUEPRINT.md` MUST NOT be reused
  when replanning; append new IDs and optionally set `supersedes: [T<old>]`.

## Replan flow

When replanning, append new tasks (new IDs) and mark old tasks superseded:

```yaml
- id: T17
  title: Replace failed checkout router
  depends_on: [T1]
  supersedes: [T4]
```

Then record the event:

```bash
codex1 replan record --mission <id> --reason <code> --supersedes T4 --json
```
