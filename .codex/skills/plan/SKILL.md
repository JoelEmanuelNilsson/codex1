---
name: plan
description: >
  Create a full Codex1 mission plan with a task DAG at the chosen level (light, medium, hard). Use when OUTCOME.md is ratified but PLAN.yaml is not yet locked, when the user explicitly asks to plan, or when a replan is required. Runs `codex1 plan choose-level`, scaffolds PLAN.yaml, fills architecture + task DAG + specs + review tasks + mission-close criteria, and validates with `codex1 plan check` before locking. At hard level, spawns explorer/advisor/plan-reviewer subagents for evidence and critique.
---

# Plan

## Overview

`$plan` produces a full mission plan for Codex1, not only a DAG. The output is a locked `PLAN.yaml` under `PLANS/<mission-id>/` that `codex1 plan check` accepts, plus one `specs/T<id>/SPEC.md` per executable or review task. The plan captures outcome interpretation, architecture, task DAG, proof strategy, planned review tasks, and mission-close criteria.

`$plan` does not execute tasks, does not record review findings, and does not close the mission.

## Preconditions

Before running `$plan`:

- `PLANS/<mission-id>/OUTCOME.md` exists and is ratified.
- `codex1 --json status` shows `outcome.ratified: true` (verdict is not `needs_user` for the outcome).

If the outcome is not ratified, stop and hand off to `$clarify`.

## Required workflow

### 1. Choose planning level

```bash
codex1 --json plan choose-level --level <light|medium|hard>
```

Record both `requested_level` and `effective_level` from the envelope. Escalate to `hard` when the mission touches any of:

- global hooks or mission-close behavior
- destructive actions, secrets, money, or external deploys
- multi-agent coordination or role boundaries
- architecture that spans subsystems
- new CLI contracts or schema changes

When escalating, pass the escalated level explicitly on the next command and include an `escalation_reason` string in the plan.

### 2. Scaffold PLAN.yaml

```bash
codex1 --json plan scaffold --level <effective-level>
```

This writes a `PLAN.yaml` skeleton with fill markers and creates `specs/` placeholders. Do not edit `STATE.json` or `EVENTS.jsonl` directly.

### 3. Fill PLAN.yaml

Required sections (canonical shape in `docs/codex1-rebuild-handoff/03-planning-artifacts.md`):

```yaml
mission_id: <id>
planning_level:
  requested: <level>
  effective: <level>
  escalation_reason: "..."   # only when effective != requested

outcome_interpretation:
  summary: |
    Concrete restatement of what the mission will deliver.

architecture:
  summary: |
    How the system will be shaped. Name the subsystems, interfaces, and data flows.
  key_decisions:
    - "..."

planning_process:
  evidence:
    - kind: explorer | advisor | docs_lookup | plan_review | direct_reasoning
      actor: "<id>"
      summary: "..."
      required_for_hard: true   # set true on hard-level missions

tasks:
  # Executable task (design | code | docs | test | research | repair):
  - id: T1
    title: "..."
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
    read_paths: []
    write_paths: []
    exclusive_resources: []
    unknown_side_effects: false
    acceptance:
      - "..."
    proof:
      - "..."

  # Review task:
  - id: T2
    title: "Review ..."
    kind: review
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    review_target:
      tasks: [T1]
    review_profiles:
      - code_bug_correctness
      - integration_intent

risks:
  - risk: "..."
    mitigation: "..."

mission_close:
  criteria:
    - "..."
```

Fill every section. No fill markers, no empty required fields.

### 4. Hard-level evidence (hard only)

On `hard`, spawn subagents and record at least one `explorer` (if repo context is unclear), one `advisor`, and one `plan_review` entry before locking. Read `references/hard-planning-evidence.md` for ready-to-paste spawn prompts and model choices.

### 5. Write task SPECs

For every executable task, write `specs/T<id>/SPEC.md` with:

- Task goal.
- Relevant context and references.
- Allowed read/write paths.
- Implementation steps or notes.
- Acceptance criteria.
- Proof commands.
- Review expectations.

For review tasks, SPEC.md lists `review_target.tasks`, the review profiles to run, and how reviewers should interpret the target (what "clean" means for this boundary).

### 6. Validate with CLI

```bash
codex1 --json plan check
```

If the envelope returns `PLAN_INVALID`, `DAG_CYCLE`, or `DAG_MISSING_DEP`, read `context` to find the bad task, dependency, or cycle. Repair and re-run until the envelope is `ok: true`. Only a passing `plan check` locks the plan.

### 7. Sanity-check the DAG (optional)

```bash
codex1 --json plan graph --format mermaid
codex1 --json plan waves
```

Confirm the derived waves match the intended execution order and that no wave is unsafely parallel.

## Task DAG rules

Every task has:

- `id` — `T1`, `T2`, ..., unique across the mission, never reused (replans append new IDs).
- `title` — one short line.
- `kind` — one of `design | code | docs | test | research | repair | review`.
- `depends_on` — explicit array. Use `[]` for roots.
- `spec` — path to `specs/T<id>/SPEC.md`.

Executable tasks (`design | code | docs | test | research | repair`) additionally declare:

- `read_paths`, `write_paths`, `exclusive_resources`, `unknown_side_effects`.
- `acceptance` — testable criteria.
- `proof` — commands or artifacts that prove acceptance.
- Any of `generated_paths`, `shared_state`, `commands`, `external_services`, `env_mutations`, `package_manager_mutation`, `schema_or_migration` that apply.

Review tasks declare `review_target.tasks` (the tasks under review) and `review_profiles` (subset of `code_bug_correctness`, `local_spec_intent`, `integration_intent`, `plan_quality`, `mission_close`). Review tasks must not declare `write_paths`.

See `references/dag-quality.md` for DAG design heuristics (root independence, exclusive resources, parallel-safe waves, review placement).

## Planned review tasks

Insert a review task in the DAG:

- After any meaningful code slice that lands a user-facing capability.
- At the end of a wave of tasks that interact with shared state.
- At subsystem completion boundaries.
- After high-risk workflow changes (hooks, mission-close, destructive actions, secrets).
- At integration boundaries between subsystems.

Mission-close review is mandatory at the end even if no review task is listed there. Include a final review task or rely on the mission-close loop, but make it explicit in `mission_close.criteria`.

## Replan

If `$plan` is invoked during execution for a replan:

- Append new tasks with new IDs; never reuse IDs.
- Record the replan with `codex1 replan record --reason <code> --supersedes <id>` (pass `--supersedes` once per task being superseded; `--reason` is mandatory). Valid reason codes live in `crates/codex1/src/cli/replan/triggers.rs::ALLOWED_REASONS` (e.g. `six_dirty`, `scope_change`, `user_request`).
- Re-run `codex1 --json plan check` before locking.
- Update `planning_process.evidence` with a new `advisor` or `plan_review` entry that explains why the replan is needed.

## Do not

- Do not execute tasks from `$plan`. Hand off to `$execute`.
- Do not record review findings from `$plan`. That belongs to `$review-loop` and the main thread.
- Do not close the mission. That belongs to `$close` and `codex1 close complete`.
- Do not store waves inside `PLAN.yaml`. Waves are derived.
- Do not edit `STATE.json` or `EVENTS.jsonl` directly.

## Resources

- `references/hard-planning-evidence.md` — spawn prompt templates for explorer, advisor, and plan-reviewer subagents, with model recommendations.
- `references/dag-quality.md` — DAG design heuristics for root tasks, parallel-safe waves, exclusive resources, and review-task placement.
