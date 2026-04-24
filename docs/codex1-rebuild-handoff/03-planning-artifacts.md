# 03 Planning And Artifacts

This file defines the mission files, outcome contract, adaptive planning process, normal-plan contract, graph-plan contract, derived waves, and review-task model.

Canonical details added after this file:

- Ralph stop behavior: `06-ralph-stop-hook-contract.md`
- Review/repair/replan behavior: `07-review-repair-replan-contract.md`
- State revisions, status verdicts, and graph/wave derivation: `08-state-status-and-graph-contract.md`
- Implementation exactness, hook config snippets, doctor checks, and first
  vertical slice: `09-implementation-errata.md`

If this file disagrees with those files on those topics, the later canonical
files win.

## Artifact Strategy

Codex1 should not create paperwork because paperwork feels serious.

Use no durable state unless it protects intent. When durable state is useful, use the lightest durable state that can do the job.

```text
normal
  lightweight planning for ordinary work
  can be chat-only when durable state adds no value
  can use durable outcome and plan/checklist for ordinary multi-step work

graph
  durable outcome, task graph, specs, proof records, review records
  useful for large/risky/multi-agent work
```

## File Layout

Use visible files under `PLANS/<mission-id>/` when durable state is needed.

Full layout:

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

Normal-mode missions may use a smaller practical subset:

```text
PLANS/<mission-id>/
  OUTCOME.md
  PLAN.yaml
  STATE.json
  EVENTS.jsonl
  CLOSEOUT.md
```

Do not use `.ralph` for mission truth.

`Ralph` is a hook behavior, not a mission-state directory.

## Artifact Ownership

| File | Owns |
| --- | --- |
| `OUTCOME.md` | Clarified destination truth |
| `PLAN.yaml` | Current route, planning mode, acceptance, validation, normal steps or graph tasks |
| `STATE.json` | Current operational state |
| `EVENTS.jsonl` | Append-only audit trail |
| `specs/T*/SPEC.md` | Graph task-local instructions |
| `specs/T*/PROOF.md` | Graph task proof receipts |
| `reviews/*.md` or `reviews/*.json` | Main-thread-recorded review outcomes |
| `CLOSEOUT.md` | Final terminal summary for durable missions |

Do not create extra truth surfaces unless clearly necessary.

## OUTCOME.md Contract

The outcome must be specified enough that a future Codex thread can understand the mission without hidden chat context.

Do not include global workflow policy as destination truth:

- global approval policy
- global autonomy rules
- model/provider configuration

Clarify unavoidable user-owned external inputs before mission lock. A locked
mission should not routinely bounce back to the user during execution.

Examples that should be clarified before lock when relevant:

- Required credentials/accounts that are not available to Codex.
- Required cost/tier upgrades that the user must perform.
- Deployments or irreversible operations outside the Git-managed repository.

Version-controlled repo edits inside the locked mission scope or assigned write
paths are not user questions by default. Codex can make them and prove them, but
it must not overwrite user work or silently broaden file ownership when the safe
scope is unclear.

Canonical `OUTCOME.md` shape is YAML frontmatter with required typed fields plus
optional human-readable markdown after the frontmatter. The CLI must parse only
the structured frontmatter for required state and digest computation. Pure YAML
may be accepted as an import convenience, but v1 writers should emit
frontmatter.

Required fields:

```yaml
schema_version: codex1.outcome.v1
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

pr_intent:
  open_pr: false
  target_branch: null
  notes: "Open a PR only if this is explicitly ratified as true."

known_risks:
  - ...

user_only_actions:
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
  $clarify, $plan, $execute, $review-loop, $interrupt, and $autopilot. These
  skills use a small deterministic CLI when durable mission state is useful.
  The workflow supports normal planning for ordinary work and graph planning
  for large/risky work. Ralph only reads codex1 status and never orchestrates.

must_be_true:
  - The user-facing product is skills-first.
  - The CLI is deterministic, small, and composable.
  - Normal work can avoid durable mission state when durable state adds no value.
  - Normal planning uses acceptance criteria, checklist, and validation.
  - Graph planning uses task IDs, depends_on arrays, derived waves, and planned review tasks.
  - Waves are derived and not stored as editable truth.
  - Reviewers return findings to the main thread.
  - Review findings include official Codex-style confidence fields.
  - Review findings are observations until triaged; only accepted blocking findings can block progress.
  - Still-dirty review boundaries after repair budget trigger autonomous replan.
  - The main thread records review results.
  - Role boundaries are prompt-governed, not fake CLI identity checks.
  - Custom subagent roles disable Codex hooks so Ralph applies only to the main/root orchestrator.
  - $interrupt pauses the active loop so the user can talk.
  - Ralph only asks codex1 status whether stop is allowed.
  - Ralph blocks at most once per Stop-hook continuation cycle.

success_criteria:
  - A fresh durable mission can be initialized under PLANS/<mission-id>/.
  - $clarify can produce a ratified OUTCOME.md with no fill markers and no vague sections.
  - $plan can produce a valid normal plan without graph-only fields.
  - $plan can produce a valid graph plan with task graph, specs, proof strategy, review tasks, and mission-close criteria.
  - codex1 plan check rejects missing graph depends_on, duplicate task IDs, unknown dependencies, and cycles in graph mode.
  - codex1 plan waves derives waves from the graph.
  - $execute can run a normal step, ready graph task, or safe ready graph wave.
  - Worker subagents can implement assigned tasks using task packets.
  - Planned review tasks spawn reviewer subagents.
  - The main thread records review clean/findings through codex1 review record.
  - Repeated dirty reviews after repair budget or invalidated assumptions trigger autonomous replan.
  - $interrupt pauses the loop so the user can talk.
  - codex1 status reports whether Ralph should allow stop.
  - codex1 doctor proves install-time Codex integration assumptions without writing mission state.
  - codex1 ralph stop-hook allows stop when stop_hook_active is true.
  - Mission-close review runs before close complete for graph/large/risky missions.
  - codex1 close check and codex1 status agree about terminal readiness.

non_goals:
  - Do not build a wrapper runtime around Codex.
  - Do not build fake permission machinery for subagents.
  - Do not use .ralph as mission state.
  - Do not store waves as editable truth.
  - Do not make reviewers write review records directly.
  - Do not expose $finish or $complete as user skills.
  - Do not make users operate a complex CLI manually.
```

## Clarify Process

`$clarify` should ask enough questions to fill `OUTCOME.md` when a durable outcome is needed.

It should ask when:

- Destination can be interpreted multiple ways.
- Success criteria are not testable.
- Non-goals are missing for broad work.
- Constraints are implied but not explicit.
- Terms like "simple", "perfect", "reliable", "done", "thorough", or "not overengineered" are used without definition.
- Irreversible external actions, deploys, production migrations, secrets, money,
  account tier upgrades, external systems, or non-Git-managed destructive
  actions are involved.

It should not ask pointless questions. Infer obvious implementation details only
after outcome truth is clear. During `$clarify` and `$autopilot` clarify phase,
ask all questions needed to ratify user-owned outcome decisions; do not replace
those decisions with assumptions.

Ratification rule:

```text
No fill markers.
No empty required fields.
No boilerplate placeholders.
No vague "works well" style success criteria.
```

## PLAN.yaml Contract

The plan is a theory of how to preserve intent through execution and repair.

Required common plan sections:

```yaml
mission_id: codex1-rebuild
planning_mode: normal | graph

planning_level:
  requested: medium
  effective: medium

outcome_interpretation:
  summary: "..."

approach:
  summary: "..."
  key_decisions:
    - "..."

acceptance_criteria:
  - "..."

validation:
  checks:
    - "..."
  manual_inspection:
    - "..."

risks:
  - risk: "..."
    mitigation: "..."

completion:
  criteria:
    - "..."
```

## Normal Plan

Normal mode should be lightweight.

Use it when the mission is ordinary multi-step work that needs enough state to avoid drift, but does not need dependency graph machinery.

Normal `PLAN.yaml` shape:

```yaml
schema_version: codex1.plan.v1
planning_mode: normal

steps:
  - id: S1
    title: "Implement filtered list behavior"
    acceptance:
      - "List filters by selected status."
    proof:
      - "npm test -- filter-list"

review:
  required: false
  notes: "Main-thread diff inspection and tests are enough unless blast radius grows."
```

Mutable step status belongs in `STATE.json`, not `PLAN.yaml`. If a scaffold
includes initial status for readability, treat it as a template hint only.

Normal mode may still use subagents, but delegation should be bounded by responsibility area rather than graph dependency.

Normal mode should not require:

- `depends_on`
- derived waves
- planned review tasks
- mission-close review
- task spec folders for every step

Normal mode can escalate to graph mode when dependency order, parallelism, repair boundaries, or review timing become important.

## Graph Plan

Graph mode is for large/risky/multi-agent work.

It adds a task graph inside the full plan.

Graph `PLAN.yaml` shape:

```yaml
schema_version: codex1.plan.v1
planning_mode: graph

planning_process:
  evidence:
    - kind: explorer | advisor | docs_lookup | plan_review | direct_reasoning
      summary: "..."
      required_for_graph: true

tasks:
  - id: T1
    title: "..."
    kind: design
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

mission_close:
  criteria:
    - "..."
```

Every graph task:

- Has `id`.
- Has `title`.
- Has `kind`.
- Has `depends_on`.
- Has `spec`.
- Uses `depends_on: []` if root.
- References existing task IDs.

Executable graph tasks should also declare:

- `read_paths`
- `write_paths`
- `exclusive_resources`
- `unknown_side_effects`
- side-effect declarations when relevant:
  - `generated_paths`
  - `shared_state`
  - `commands`
  - `external_services`
  - `env_mutations`
  - `package_manager_mutation`
  - `schema_or_migration`
- `proof`
- review expectations or a downstream planned review task

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

## Planning Levels

Planning level changes process depth.

```text
light  = small/local/obvious work
medium = normal multi-step work
hard   = architecture/risky/autonomous/multi-agent work
```

Planning level should be selected through a CLI-supported handshake when durable planning is needed.

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
light  -> direct planning is acceptable
medium -> normal planning with enough context gathering and internal critique
hard   -> deeper planning loop with exploration, critique, plan review, and validation before lock
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

## Graph Planning Process

For graph/hard planning, before locking the plan, the main thread should usually:

- Use the single explorer role when repo or system context is unclear.
- Use docs lookup when external APIs/libraries matter.
- Use advisor/critique subagents when design risk matters.
- Use plan-review subagents when plan quality materially affects correctness.
- Validate the graph.
- Validate proof strategy.
- Validate review tasks.
- Confirm the first executable wave.

Do not make this a bureaucracy. The purpose is to ensure large/risky planning gets the extra critique it needs.

## Derived Waves

Graph mode only.

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

Wave eligibility:

- All dependencies are complete or review-clean.
- Task is pending and dependency-satisfied.
- No mandatory replan trigger is open for that task.
- Required spec/proof/plan freshness checks pass.

Repairs are not wave members. A current review boundary in `repair_required`
becomes `next_action.kind = repair` in status, not a derived graph wave task.

## Review Tasks

Review timing should be risk-scaled.

Normal work:

- Formal reviewer optional.
- Small/local normal work usually uses direct main-thread verification.
- Add review only when risk, ambiguity, or blast radius justifies it.

Graph work:

- Plan review tasks after meaningful code slices, waves of interacting tasks, subsystem completion, high-risk workflow changes, and integration boundaries.
- Mission-close review is mandatory for large/risky graph missions even if no review task exists at the end.

Review records should support official Codex-style confidence fields:

```yaml
findings:
  - title: "..."
    priority: 1
    confidence_score: 0.82
overall_confidence_score: 0.77
```

## Specs

Every executable graph task should have:

```text
specs/T<ID>/SPEC.md
```

`SPEC.md` should contain:

- Task goal.
- Relevant context.
- Allowed read/write paths.
- Steps or implementation notes.
- Acceptance criteria.
- Proof commands.
- Review expectations.

Workers read the spec. Reviewers may read relevant specs through review packets.

Normal-mode steps may inline their spec in `PLAN.yaml` unless the step is complex enough to deserve a separate file.
