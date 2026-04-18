# Codex1 V2 Architecture Brief

Status: draft for critique
Date: 2026-04-18

## Thesis

Codex1 V2 should be a skills-first Codex product backed by a small,
composable CLI contract kernel.

The user should experience Codex1 through native Codex skills:

- `$clarify`
- `$plan`
- `$execute`
- `$review-loop`
- `$autopilot`
- `$close`

The deterministic backend should be a boring `codex1` CLI that validates files,
derives status, computes task waves, records task/review state, and exposes
stable JSON.

Ralph should be thin. Ralph should not understand the mission model deeply. It
should call:

```bash
codex1 status --mission <id> --json
```

Then it should block only when that status says an active parent loop has an
actionable next step.

Subagents should be ordinary bounded Codex agents. They should never be inside
Ralph. They should never own mission truth.

## Why This Exists

The previous Codex1 design proved the right high-level product, but the runtime
became too heavy:

- too many truth surfaces
- too many gate/fingerprint interactions
- too much parent authority token fragility
- stale reviewer lanes that kept talking after their boundary moved
- Ralph logic that became too smart and sometimes blocked the wrong thing
- review writeback complexity that made the user feel like they were operating
  the machine, not using the product

The lesson is not "remove contracts." The lesson is "center the contracts in a
small command-shaped kernel and keep the public workflow readable."

Codex is very good at using precise commands. OpenAI's agent-friendly CLI
guidance says strong agent tools should have help screens, stable JSON,
predictable errors, setup checks, small default outputs, and file exports for
large payloads:

- https://developers.openai.com/codex/use-cases/agent-friendly-clis

Codex1 V2 should be built around that.

## Product Split

| Layer | Owns | Must Not Own |
| --- | --- | --- |
| Skills | User-facing workflow, tone, interviewing, planning posture, orchestration instructions | Hidden state, gate math, stale truth derivation |
| CLI contract kernel | Validation, status projection, task DAG checks, wave derivation, review cleanliness, close legality | Architecture choice, user intent, prose judgment |
| Ralph | Stop/resume guard over `codex1 status --mission <id> --json` | Planning, reviewing, executing, subagent management |
| Subagents | Bounded scouting, writing, reviewing, critique | Parent loop, mission close, gate clearing, hidden state |
| Visible files | Destination, route, task specs, proofs, review outputs, event history | Chat-only truth |

The product should feel native to Codex. The user should not need to remember
CLI flags. Codex should use the CLI underneath the skills.

## Core User Flow

Manual flow:

```text
$clarify -> $plan -> $execute -> review/repair/replan loop -> mission close
```

Autopilot flow:

```text
$autopilot -> clarify -> plan -> execute -> review/repair/replan loop -> mission close
```

Pause/discussion flow:

```text
$close -> parent loop paused -> user talks -> $execute or $autopilot resumes
```

## Canonical Files

Keep the file model small.

```text
PLANS/<mission-id>/
  OUTCOME-LOCK.md
  PROGRAM-BLUEPRINT.md
  STATE.json
  events.jsonl
  reviews/
    <bundle-id>.json
    outputs/
      <output-id>.json
  specs/
    T1/
      SPEC.md
      PROOF.md
      REVIEW.md
    T2/
      SPEC.md
      PROOF.md
      REVIEW.md
```

Canonical meanings:

- `OUTCOME-LOCK.md` is destination truth.
- `PROGRAM-BLUEPRINT.md` is route truth and contains the immutable task DAG.
- `STATE.json` is authoritative operational state for V2.
- `events.jsonl` is append-only audit history for V2. It is not a replay
  authority unless replay semantics are explicitly specified.
- `specs/T*/SPEC.md` is bounded task detail.
- `specs/T*/PROOF.md` records proof evidence.
- `specs/T*/REVIEW.md` records human-readable review summary.
- `reviews/*.json` and `reviews/outputs/*.json` are machine-readable review
  truth.

Avoid many competing markdown artifacts. Human-readable docs are welcome, but
there should be few canonical machine truth surfaces.

Single-source rule:

- blueprint owns task identity, dependencies, scopes, proof requirements, and
  review requirements
- `STATE.json` owns mutable task status and parent-loop status
- review JSON owns machine review evidence
- markdown proof/review files are human-readable evidence refs
- `events.jsonl` records an audit trail, but must not disagree with
  `STATE.json`

## Planning Levels

Use three public planning levels:

| Level | Name | Meaning |
| --- | --- | --- |
| `light` | 1 | Small, local, obvious work |
| `medium` | 2 | Normal multi-step or multi-file work |
| `hard` | 3 | Architecture, risky, autonomous, long-running, or multi-agent work |

The requested level is not always the effective level.

```yaml
requested_level: light
risk_floor: hard
effective_level: hard
```

Codex1 may escalate planning level when the mission touches risky or broad
surfaces such as auth, data loss, migrations, deploys, global config, many
subsystems, or long autonomous execution.

Do not recreate a five-level scale. It created ceremony. `hard` means the
serious route.

## Plan DAG Is Mandatory

Every plan must include explicit task IDs and dependencies.

```yaml
tasks:
  - id: T1
    title: Define CLI contract
    depends_on: []

  - id: T2
    title: Implement status command
    depends_on: [T1]

  - id: T3
    title: Update skills
    depends_on: [T1]

  - id: T4
    title: Integration review
    depends_on: [T2, T3]
```

A plan without a DAG is not executable. It is only a narrative.

The DAG row is the scheduling and safety contract. The full implementation
detail belongs in `specs/T*/SPEC.md`.

Minimum task row:

```yaml
- id: T2
  title: Implement status command
  kind: code
  depends_on: [T1]
  spec_ref: specs/T2/SPEC.md
  read_paths:
    - crates/codex1/**
  write_paths:
    - crates/codex1/**
  exclusive_resources:
    - status_schema
  proof:
    - cargo test -p codex1
  review_profiles:
    - code_bug_correctness
    - local_spec_intent
```

Required task fields:

- `id`
- `title`
- `kind`
- `depends_on`
- `spec_ref` for executable tasks
- `read_paths`
- `write_paths`
- `exclusive_resources`
- side-effect declarations, when applicable:
  - `generated_paths`
  - `shared_state`
  - `commands`
  - `external_services`
  - `env_mutations`
  - `package_manager_mutation`
  - `schema_or_migration`
  - `unknown_side_effects`
- `proof`
- `review_profiles`

Task IDs must never be reused within a mission. Replans append new task IDs and
may supersede old task IDs. Do not reuse `T2` in a later graph revision.

## Waves Are Derived

Waves should not be hand-authored canonical truth. They should be derived by
the CLI from the task DAG plus task state.

A task is wave-eligible when:

- all dependencies are `review_clean` or `complete`
- task status is `ready`, or task status is `needs_repair` with a current
  failed review boundary assigned for repair
- no replan-required contradiction is open for that task
- package/spec/plan freshness checks pass

Then the CLI derives candidate waves:

```bash
codex1 plan waves --mission <id> --json
```

Example output:

```json
{
  "ok": true,
  "mission_id": "example",
  "waves": [
    {
      "id": "W1",
      "tasks": ["T1"],
      "mode": "serial"
    },
    {
      "id": "W2",
      "tasks": ["T2", "T3"],
      "mode": "parallel",
      "safety": {
        "dependency_independent": true,
        "write_paths_disjoint": true,
        "read_write_conflicts": [],
        "exclusive_resources_disjoint": true
      }
    }
  ]
}
```

## Parallel Safety

`depends_on` is not enough for parallel execution.

Parallel tasks must also pass workspace safety:

- no direct or transitive dependency inside the same wave
- pairwise disjoint write paths
- no task writes a path another same-wave task reads
- disjoint exclusive resources
- no shared lockfile, schema, migration, generated output, deploy config,
  global config, package-manager state, or environment mutation
- no hidden shared review boundary that must be judged together before either
  task can become clean

Unknown side effects force serial execution.

Parallel workers must use isolated worktrees/branches or return patch artifacts
for parent integration. Same-worktree parallel writes are serial by default
unless a plan explicitly proves isolation.

Simple rule:

```text
depends_on gives correctness order.
wave safety gives workspace order.
Both must pass for parallel execution.
```

## Execution Model

`$execute` should ask the CLI what is next.

```bash
codex1 status --mission <id> --json
codex1 task next --mission <id> --json
codex1 task start --mission <id> T2 --json
```

After implementation and proof:

```bash
codex1 task finish --mission <id> T2 --proof specs/T2/PROOF.md --json
```

Finishing a task does not make it complete. It makes review owed.

Task lifecycle:

```text
planned -> ready -> in_progress -> proof_submitted -> review_owed
review_owed -> review_clean -> complete
review_owed -> review_failed -> needs_repair
needs_repair -> in_progress
needs_repair -> replan_required
```

Implementation alone never satisfies dependencies. Dependencies should be
satisfied only by `review_clean` or explicit `complete`.

## Review Model

Review attaches to tasks, waves, or mission close.

```bash
codex1 review open --mission <id> --task T2 --profiles code_bug_correctness,local_spec_intent --json
codex1 review submit --mission <id> --bundle B1 --input reviewer-output.json --json
codex1 review status --mission <id> --bundle B1 --json
codex1 review close --mission <id> --bundle B1 --json
```

Review rules:

- parent does not self-review
- reviewers never clear gates
- reviewer output is durable evidence
- parent integrates reviewer outputs through the CLI
- missing required reviewer output is not clean
- required reviewer cardinality is explicit per review requirement
- one reviewer cannot satisfy multiple requirements unless the requirement says
  so
- any P0/P1/P2 finding is not clean
- six consecutive non-clean loops for the same boundary routes to replan

Review profiles:

| Profile | Use |
| --- | --- |
| `code_bug_correctness` | code-producing task |
| `local_spec_intent` | single task/spec intent |
| `integration_intent` | coupled tasks/waves |
| `mission_close` | final close review |

Blueprints should include machine-readable review boundaries when review is not
single-task local review:

```yaml
review_boundaries:
  - id: RB1
    kind: integration
    tasks: [T2, T3]
    depends_on_clean: [T2, T3]
    requirements:
      - id: RB1-intent
        profile: integration_intent
        min_outputs: 1
        allowed_roles: [reviewer]
```

## Replan Model

Keep replan simple.

Local repair:

```text
same task, same spec, same scope
```

Replan:

```text
append event
add new task IDs
optionally supersede old tasks
do not erase history
```

Example:

```yaml
- id: T5
  title: Replace failed setup approach
  depends_on: [T1]
  supersedes: [T2]
```

Mandatory replan triggers:

- write scope expansion
- false or missing dependency
- interface contract change
- impossible or proxy proof row
- review contract change
- outcome meaning change
- six consecutive non-clean review loops
- parallel safety assumption invalidated

## Ralph Model

Ralph should be tiny.

At stop:

```bash
codex1 status --mission <id> --json
```

If status says:

```json
{
  "parent_loop": {
    "active": true
  },
  "stop_policy": {
    "allow_stop": false,
    "reason": "active_parent_loop"
  },
  "next_action": {
    "kind": "start_task",
    "display_message": "Continue task T2"
  }
}
```

then Ralph blocks with that next action.

If status says:

```json
{
  "parent_loop": {
    "active": false
  },
  "stop_policy": {
    "allow_stop": true,
    "reason": "discussion_mode"
  }
}
```

then Ralph allows stop.

Ralph must not:

- plan
- execute
- review
- spawn subagents
- infer mission completion
- inspect hidden state not exposed through `codex1 status`
- block reviewer/subagent lanes

## Subagent Model

Subagents are normal bounded workers or reviewers.

They receive:

- mission path
- task ID or review bundle ID
- explicit read/write scope
- output contract
- forbidden actions

They do not receive:

- Ralph lease authority
- mission close authority
- permission to clear review
- permission to rewrite the blueprint unless explicitly assigned by parent

Worker outputs:

- patch or changed files
- proof evidence
- notes

Reviewer outputs:

- `NONE`
- findings with severity, evidence refs, rationale, suggested action

Late outputs after supersession should be quarantined as stale, not allowed to
confuse current truth.

Every worker/reviewer output must carry enough binding data to make staleness
machine-checkable:

- `task_id` or `bundle_id`
- `task_run_id` for worker outputs
- `graph_revision`
- `state_revision` or `expected_seq`
- evidence snapshot hash or proof hash
- parent-issued packet id

If any binding no longer matches current truth, the CLI records or reports
`STALE_OUTPUT` and refuses to count the output as current evidence.

## Mission Close

Mission close requires:

- all non-superseded required tasks are `review_clean` or `complete`
- no open P0/P1/P2 findings
- all proof rows have receipts
- coupled task groups have integration review
- mission-close review has required reviewer outputs
- `codex1 mission-close check --mission <id> --json` passes

Only then can `STATE.json` become terminal:

```json
{
  "phase": "complete",
  "terminality": "terminal",
  "verdict": "complete"
}
```

## Non-Goals

Codex1 V2 should not start with:

- hidden daemon
- database-backed mission runtime
- external wrapper that controls Codex
- custom subagent framework
- graph UI
- stored waves as canonical truth
- complex multi-token authority protocol
- markdown-only machine truth
- many competing artifacts
- old-style giant runtime before the CLI contract is proven

## Full Product Scope

V2 is not an MVP. The target is the complete Codex1 workflow:

```text
clarify -> plan DAG -> execute waves -> proof -> review-loop -> repair/replan
-> pause/resume -> autopilot -> mission close
```

The implementation should still proceed through dependency waves because the
product itself depends on DAG discipline.

## Build Waves

Wave 1 proves the kernel every later feature depends on:

```bash
codex1 init
codex1 status --mission <id> --json
codex1 plan check --mission <id> --json
codex1 plan waves --mission <id> --json
codex1 task next --mission <id> --json
```

Wave 1 acceptance:

- can create a mission folder
- can parse `OUTCOME-LOCK.md`
- can parse `PROGRAM-BLUEPRINT.md` task DAG
- can reject malformed DAGs
- can derive ready tasks and waves
- can emit stable status JSON
- Ralph can use only `codex1 status --mission <id> --json`

Wave 2 builds task execution and proof:

- `codex1 task start`
- `codex1 task finish`
- task run IDs
- state revision checks
- proof refs
- stale worker-output quarantine

Wave 3 builds review and repair/replan:

- `codex1 review open`
- `codex1 review submit`
- `codex1 review status`
- `codex1 review close`
- repair assignment
- six-loop replan routing
- stale reviewer-output quarantine

Wave 4 builds parent-loop UX:

- `$close` pause/resume
- Ralph stop hook as `codex1 status --mission <id> --json`
- `$execute` composition
- `$review-loop` composition

Wave 5 builds complete autonomy:

- `$autopilot`
- advisor/CritiqueScout checkpoint handling
- mission-close review
- end-to-end qualification

This sequencing is not product scope reduction. It is dependency-ordering. The
full acceptance target remains the complete workflow.
