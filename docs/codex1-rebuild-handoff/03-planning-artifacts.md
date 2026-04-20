# 03 Planning And Artifacts

This file defines the mission files, outcome contract, planning process, DAG contract, derived waves, and review-task model.

## File Layout

Use visible files under `PLANS/<mission-id>/`.

```text
PLANS/<mission-id>/
  OUTCOME.md
  PLAN.yaml
  STATE.json
  EVENTS.jsonl
  specs/
    T1/
      SPEC.md
      PROOF.md
    T2/
      SPEC.md
      PROOF.md
  reviews/
    R1.md
    R2.md
  CLOSEOUT.md
```

Do not use `.ralph` for mission truth.

`Ralph` is a hook behavior, not a mission-state directory.

## Artifact Ownership

| File | Owns |
| --- | --- |
| `OUTCOME.md` | Clarified destination truth |
| `PLAN.yaml` | Route, architecture, task DAG, review tasks |
| `STATE.json` | Current operational state |
| `EVENTS.jsonl` | Append-only audit trail |
| `specs/T*/SPEC.md` | Task-local instructions |
| `specs/T*/PROOF.md` | Task proof receipts |
| `reviews/*.md` | Main-thread-recorded review outcomes |
| `CLOSEOUT.md` | Final terminal summary |

Do not create extra truth surfaces unless clearly necessary.

## OUTCOME.md Contract

The outcome must be overly specified.

A future Codex thread should understand the mission without hidden chat context.

Do not include:

- `approval_boundaries`
- `autonomy`

Approval and autonomy are global workflow/safety rules, not mission destination truth.

Recommended `OUTCOME.md` shape can be YAML frontmatter plus readable markdown, or pure YAML. The important part is that the CLI can check required fields.

Required fields:

```yaml
mission_id: codex1-rebuild
status: draft | ratified
title: "..."

original_user_goal: |
  ...

interpreted_destination: |
  ...

must_be_true:
  - ...

success_criteria:
  - ...

non_goals:
  - ...

constraints:
  - ...

definitions:
  term: "meaning"

quality_bar:
  - ...

proof_expectations:
  - ...

review_expectations:
  - ...

known_risks:
  - ...

resolved_questions:
  - question: "..."
    answer: "..."
```

### Bad Outcome Example

```yaml
interpreted_destination: "Codex1 works well."
success_criteria:
  - "Workflow is reliable."
non_goals:
  - "Don't overengineer."
```

This is unacceptable. It leaves too much for another agent to infer.

### Good Outcome Example

```yaml
interpreted_destination: |
  Codex1 is rebuilt as a native Codex workflow where users interact through
  $clarify, $plan, $execute, $review-loop, $close, and $autopilot. These
  skills use a small deterministic CLI to store and validate visible mission
  files, execute DAG-based plans, derive waves, record main-thread review
  outcomes, pause/resume loops, and close missions only after planned work and
  mission-close review are clean.

must_be_true:
  - The user-facing product is skills-first.
  - The CLI is deterministic, small, and composable.
  - Plans contain tasks with explicit IDs and depends_on arrays.
  - Waves are derived and not stored as editable truth.
  - Review timing is mostly represented by review tasks in the DAG.
  - Reviewers return findings to the main thread.
  - The main thread records review results.
  - Role boundaries are prompt-governed, not fake CLI identity checks.
  - Ralph only asks codex1 status whether stop is allowed.

success_criteria:
  - A fresh mission can be initialized under PLANS/<mission-id>/.
  - $clarify can produce a ratified OUTCOME.md with no fill markers and no vague sections.
  - $plan hard can produce PLAN.yaml with full plan sections, task DAG, specs, proof strategy, review tasks, and mission-close criteria.
  - codex1 plan check rejects missing depends_on, duplicate task IDs, unknown dependencies, and cycles.
  - codex1 plan waves derives waves from the DAG.
  - $execute can run a ready task or safe ready wave.
  - Worker subagents can implement assigned tasks using task packets.
  - Planned review tasks spawn reviewer subagents.
  - The main thread records review clean/findings through codex1 review record.
  - Six consecutive dirty reviews for one active target trigger replan.
  - $close pauses the loop so the user can talk.
  - codex1 status reports whether Ralph should allow stop.
  - Mission-close review runs before close complete.
  - codex1 close check and codex1 status agree about terminal readiness.

non_goals:
  - Do not build a wrapper runtime around Codex.
  - Do not build fake permission machinery for subagents.
  - Do not use .ralph as mission state.
  - Do not store waves as editable truth.
  - Do not make reviewers write review records directly.
  - Do not make users operate a complex CLI manually.
```

## Clarify Process

`$clarify` should ask enough questions to fill `OUTCOME.md`.

It should ask when:

- Destination can be interpreted multiple ways.
- Success criteria are not testable.
- Non-goals are missing for broad work.
- Constraints are implied but not explicit.
- Terms like "simple", "perfect", "reliable", "done", "thorough", or "not overengineered" are used without definition.
- Destructive actions, deploys, migrations, secrets, money, or external systems are involved.

It should not ask pointless questions. Infer obvious things, state assumptions, and ask only when the answer changes the plan.

Ratification rule:

```text
No fill markers.
No empty required fields.
No boilerplate placeholders.
No vague "works well" style success criteria.
```

## PLAN.yaml Contract

The plan is a full mission plan. The DAG is only the execution graph inside it.

Required plan sections:

```yaml
mission_id: codex1-rebuild

planning_level:
  requested: hard
  effective: hard

outcome_interpretation:
  summary: "..."

architecture:
  summary: "..."
  key_decisions:
    - "..."

planning_process:
  evidence:
    - kind: explorer | advisor | docs_lookup | plan_review | direct_reasoning
      summary: "..."
      required_for_hard: true

tasks:
  - id: T1
    title: "..."
    kind: design
    depends_on: []
    spec: specs/T1/SPEC.md
    acceptance:
      - "..."
    proof:
      - "..."

risks:
  - risk: "..."
    mitigation: "..."

mission_close:
  criteria:
    - "..."
```

## Planning Levels

All levels require the same basic plan structure.

The level changes process depth.

```text
light  = full structure for small/local/obvious work
medium = full structure with normal deliberation
hard   = full structure plus mandatory deeper critique/research/delegation
```

Planning level should be selected through a CLI-supported handshake.

Preferred flow:

```bash
codex1 plan choose-level
```

Interactive prompt:

```text
Choose planning level:
1. light  - small/local/obvious work
2. medium - normal multi-step work
3. hard   - architecture/risky/autonomous/multi-agent work
```

Accepted inputs:

```text
1 / light
2 / medium
3 / hard
```

Use only `light`, `medium`, and `hard` in `PLAN.yaml`, docs, and skill prompts. Numeric values are CLI aliases. Do not use `low` or `high` as planning verbs.

The selected level determines which planning workflow must run before the plan can be locked:

```text
light  -> direct planning is acceptable, with the full required plan structure
medium -> normal planning with enough context gathering and internal critique
hard   -> mandatory deeper planning loop with exploration, advisor critique, plan review, and validation before lock
```

The CLI records the requested level and scaffolds the plan accordingly. The main thread may escalate the effective level if risk demands it.

Examples:

```yaml
planning_level:
  requested: medium
  effective: medium
```

```yaml
planning_level:
  requested: light
  effective: hard
  escalation_reason: "The mission touches global hooks and mission-close behavior."
```

Only include `escalation_reason` when the main thread escalates above the requested level.

For hard planning, before locking the plan, the main thread should usually:

- Use explorer subagents when repo context is unclear.
- Use docs lookup when external APIs/libraries matter.
- Use advisor/critique subagents.
- Use plan-review subagents.
- Validate the DAG.
- Validate proof strategy.
- Validate review tasks.
- Confirm first executable wave.

If the user asks for hard:

```yaml
planning_level:
  requested: hard
  effective: hard
```

No reason is needed.

## Task DAG

Every task:

- Has `id`.
- Has `kind`.
- Has `depends_on`.
- Has `spec`.
- Uses `depends_on: []` if root.
- References existing task IDs.

Task IDs:

```text
T1, T2, T3, ...
```

Do not reuse task IDs. Replans append new tasks.

Example:

```yaml
tasks:
  - id: T1
    title: Define CLI contract
    kind: design
    depends_on: []
    spec: specs/T1/SPEC.md

  - id: T2
    title: Implement status command
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    read_paths:
      - crates/codex1/**
    write_paths:
      - crates/codex1/src/status/**
    exclusive_resources:
      - status-json-schema
    unknown_side_effects: false
    proof:
      - cargo test -p codex1 status

  - id: T3
    title: Review status command integration
    kind: review
    depends_on: [T2]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T2]
    review_profiles:
      - code_bug_correctness
      - integration_intent
```

## Derived Waves

Do not store waves in `PLAN.yaml`.

The source of truth is:

```text
tasks + depends_on + current task state
```

The CLI derives waves:

```bash
codex1 plan waves --json
```

Example:

```text
T1 depends_on []
T2 depends_on [T1]
T3 depends_on [T1]
T4 depends_on [T2, T3]

Derived:
W1 = T1
W2 = T2, T3
W3 = T4
```

Reason:

```text
One source of truth.
No stored wave drift.
Main thread can still inspect waves through CLI.
User does not need stored waves in the plan file.
```

## Review Tasks

Review timing should be planned in the DAG.

Use review tasks after:

- Meaningful code slices.
- Waves of interacting tasks.
- Subsystem completion.
- High-risk workflow changes.
- Integration boundaries.

Mission-close review is mandatory even if no review task exists at the end.

## Specs

Every executable task should have:

```text
specs/T<ID>/SPEC.md
```

SPEC.md should contain:

- Task goal.
- Relevant context.
- Allowed read/write paths.
- Steps or implementation notes.
- Acceptance criteria.
- Proof commands.
- Review expectations.

Workers read the spec. Reviewers may read relevant specs through review packets.
