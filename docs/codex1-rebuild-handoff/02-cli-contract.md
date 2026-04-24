# 02 CLI Contract

This file defines the `codex1` CLI shape.

The CLI should be designed according to the OpenAI agent-friendly CLI guidance:

- Official guide: https://developers.openai.com/codex/use-cases/agent-friendly-clis
- CLI Creator: https://github.com/openai/skills/tree/main/skills/.curated/cli-creator

Key principles from the guide:

- Start from the job Codex needs to do, not from the implementation technology.
- Give Codex a command it can run from any folder.
- Make commands composable with `rg`, `git`, tests, and repo scripts.
- Use stable JSON, predictable errors, paged/exact reads, small default output, and file exports for large payloads.
- Add a companion skill so future Codex threads know which commands to run first and which writes need approval.
- Verify the installed command from outside the source folder.

## CLI Philosophy

The CLI is a deterministic helper. It should help Codex by making durable mission state queryable and writable through exact commands.

The CLI should not be an AI. It should not decide architecture, ask semantic clarification questions, spawn subagents, detect caller identity, or force every task into a graph.

Correct split:

```text
Codex reasons.
Skills guide.
CLI validates, derives, records, and reports.
```

## Command Requirements

Every command:

- Has `--help`.
- Supports `--json`.
- Supports `--mission <mission-id>` unless it is `codex1 init` or a command explicitly creating/selecting a mission.
- Supports `--repo-root <path>` or another explicit repo-root mechanism; avoid hidden active-mission resolution.
- Emits stable JSON.
- Emits stable error codes.
- Avoids giant prose output.
- Works from any repo folder when given mission/root args or when repo root is discoverable by explicit config.
- Has no surprise interactive prompts. The exception is a command whose purpose is explicitly interactive, such as `codex1 plan choose-mode` or `codex1 plan choose-level`.

Mutating commands:

- Must be idempotent where possible.
- Must fail clearly when preconditions are missing.
- Should support `--dry-run` if implementation cost is small.
- Should support `--expect-revision <N>` or equivalent stale-writer protection.
- Should not mutate files when validation fails.

## Target Command Surface

The full v1 target command surface is:

```bash
codex1 init
codex1 status
codex1 doctor

codex1 outcome check
codex1 outcome ratify

codex1 plan choose-mode
codex1 plan choose-level
codex1 plan scaffold
codex1 plan check
codex1 plan lock
codex1 plan graph
codex1 plan waves

codex1 task next
codex1 task start
codex1 task finish
codex1 task status
codex1 task packet

codex1 review start
codex1 review packet
codex1 review record
codex1 review status

codex1 replan record
codex1 replan check

codex1 loop pause
codex1 loop resume
codex1 loop deactivate
codex1 loop activate

codex1 close check
codex1 close complete
codex1 close record-review

codex1 ralph stop-hook
```

Do not implement that whole surface at once. The first vertical slice in
`09-implementation-errata.md` implements only the subset needed for one durable
normal mission: help, init, doctor, outcome, normal plan, task/step lifecycle,
status, plan lock, loop activate/pause/resume/deactivate, Ralph, and minimal
close. Graph, waves, review, replan, and mission-close review come after that
slice works end to end.

`loop activate` is the canonical entry point other subsystems use to set `state.loop.active = true` without inventing their own loop-state mutation. Skills call it from `$execute` / `$autopilot` after a durable plan is locked.

`close record-review` records the main-thread outcome of mission-close review.
It is the only write path that transitions close state from
`mission_close_review_open` to `mission_close_review_passed`. Planned review
tasks still use `codex1 review record`; this command exists specifically
because mission-close review is a terminal boundary rather than a normal task.

`ralph stop-hook` is an internal hook adapter, not a user-facing skill. It reads Codex `Stop` hook JSON from stdin, obtains the same stop projection as `codex1 status --json`, and emits Codex Stop-hook output.

`doctor` is an install-time diagnostic command. It may verify the installed CLI,
hook config snippets, requirements snippets, model policy, and subagent hook
disabling, but it must not become runtime fallback routing or mission state.

Do not add more commands until the minimal set is excellent.

## Important Non-Features

The CLI must not:

- Ask "are you the parent or subagent?"
- Detect whether the caller is reviewer, worker, explorer, advisor, or main thread.
- Block reviewer commands based on identity.
- Spawn subagents.
- Read hidden chat state.
- Use `.ralph` mission truth.
- Store waves as editable truth.
- Require a DAG for normal-mode plans.
- Use `PreToolUse` or `PostToolUse` hooks as Ralph's state source.
- Expose a public `$finish` or `$complete` user skill.
- Become a giant workflow daemon.

Role behavior is prompt-governed. Artifact shape and state transitions are CLI-governed.

## `codex1 init`

Creates durable mission files under `PLANS/<mission-id>/`.

Recommended full layout:

```text
PLANS/
  ACTIVE.json
  <mission-id>/
    OUTCOME.md
    PLAN.yaml
    STATE.json
    EVENTS.jsonl
    specs/
    reviews/
    CLOSEOUT.md
```

Normal-mode missions may leave `specs/` and `reviews/` mostly empty. Graph-mode missions use them heavily.

`PLANS/ACTIVE.json` is repo-level selection metadata. It is not inside a
mission directory and is not mission truth.

Recommended active pointer:

```json
{
  "schema_version": "codex1.active.v1",
  "mission_id": "example",
  "selected_at": "2026-04-24T10:00:00Z",
  "selected_by": "codex1 loop activate",
  "purpose": "ralph_status_default"
}
```

The pointer may select a default mission for read/status/Ralph paths. Mutating
commands should still require an explicit mission/root argument or a command
whose purpose is selection, such as `codex1 loop activate --mission <id>`.
Do not store phase, loop, task, review, close, or terminal state in
`ACTIVE.json`; those belong in `PLANS/<mission-id>/STATE.json`.

Initial state:

```json
{
  "schema_version": "codex1.state.v1",
  "mission_id": "example",
  "revision": 0,
  "phase": "clarify",
  "planning_mode": "unset",
  "planning_level": { "requested": null, "effective": null },
  "outcome": {
    "ratified": false,
    "ratified_revision": null,
    "outcome_digest": null
  },
  "plan": {
    "locked": false,
    "locked_revision": null,
    "plan_digest": null,
    "outcome_digest_at_lock": null,
    "supersedes": []
  },
  "loop": { "active": false, "paused": false, "mode": "none" },
  "steps": {},
  "tasks": {},
  "reviews": {},
  "replan": { "required": false, "reason": null, "boundary_id": null, "supersedes": [] },
  "close": {
    "state": "not_ready",
    "requires_mission_close_review": false,
    "boundary_revision": null,
    "latest_review_id": null,
    "passed_revision": null,
    "closeout_path": "CLOSEOUT.md",
    "closeout_digest": null
  },
  "terminal": { "complete": false, "completed_revision": null }
}
```

## `codex1 status --json`

This is the most important command.

The detailed canonical status contract is in
`08-state-status-and-graph-contract.md`. If this section and that file
disagree, file `08` wins.

It is consumed by:

- Skills.
- Main thread.
- Ralph.
- Humans debugging state.

It should answer:

- Is outcome ratified?
- Which planning mode is active?
- Is the plan valid for that mode?
- What phase are we in?
- Is loop active/paused?
- What is the next action?
- What normal steps are ready?
- What graph tasks are ready?
- What ready graph wave exists?
- Is ready graph wave parallel-safe?
- Are review tasks ready?
- Is replan required?
- Is close ready?
- Should Ralph allow stop?

`verdict` is the primary status field. `phase`, `planning_mode`, `loop`,
`next_action`, `close`, and `stop` must be internally consistent with the
verdict.

Post-lock verdict values:

```text
inactive
paused
invalid_state
continue_required
replan_required
close_required
complete
```

Do not use `needs_user`, `blocked_external`, or `validation_required` as normal
post-lock execution verdicts.

`next_action` must say whether the next action is Codex-owned and autonomous:

```json
{
  "kind": "run_wave",
  "owner": "codex",
  "required": true,
  "autonomous": true
}
```

When Codex cannot continue autonomously, use a non-blocking next action rather
than `needs_user` or `blocked_external`:

```json
{
  "kind": "explain_and_stop",
  "owner": "codex",
  "required": false,
  "autonomous": false,
  "reason": "missing_required_credentials",
  "message": "Codex cannot continue autonomously because required credentials are unavailable in this environment."
}
```

Ralph must allow stop for `autonomous: false`. The main thread should explain
the situation plainly and stop; it should not keep looping or invent a helpless
mission state.

Normal-mode example:

```json
{
  "ok": true,
  "schema_version": "codex1.status.v1",
  "mission_id": "normal-feature",
  "mission_root": "/abs/path/PLANS/normal-feature",
  "state_revision": 12,
  "outcome_digest": "sha256:...",
  "plan_digest": "sha256:...",
  "planning_mode": "normal",
  "phase": "execute",
  "verdict": "continue_required",
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "next_action": {
    "kind": "run_step",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "step_id": "S2",
    "title": "Implement filtered list behavior"
  },
  "ready_steps": ["S2"],
  "ready_tasks": [],
  "ready_wave": null,
  "reviews": {
    "pending_boundaries": [],
    "accepted_blocking_count": 0
  },
  "replan": {
    "required": false,
    "reason": null
  },
  "close": {
    "ready": false,
    "required": false,
    "requires_mission_close_review": false
  },
  "stop": {
    "allow": false,
    "reason": "block_active_normal_step",
    "mode": "soft",
    "message": "Codex1 says required work remains: continue step S2: Implement filtered list behavior.\nContinue that now, or use $interrupt / codex1 loop pause to stop intentionally.\nIf this is a false positive, explain briefly and stop; Ralph will not block again in this turn."
  }
}
```

Graph-mode example:

```json
{
  "ok": true,
  "schema_version": "codex1.status.v1",
  "mission_id": "codex1-rebuild",
  "mission_root": "/abs/path/PLANS/codex1-rebuild",
  "state_revision": 18,
  "outcome_digest": "sha256:...",
  "plan_digest": "sha256:...",
  "planning_mode": "graph",
  "phase": "execute",
  "verdict": "continue_required",
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "next_action": {
    "kind": "run_wave",
    "owner": "codex",
    "required": true,
    "autonomous": true,
    "wave_id": "W2",
    "tasks": ["T2", "T3"]
  },
  "ready_steps": [],
  "ready_tasks": ["T2", "T3"],
  "ready_wave": {
    "wave_id": "W2",
    "tasks": ["T2", "T3"],
    "parallel_safe": true,
    "parallel_blockers": []
  },
  "reviews": {
    "pending_boundaries": [],
    "accepted_blocking_count": 0
  },
  "replan": {
    "required": false,
    "reason": null
  },
  "close": {
    "ready": false,
    "required": false,
    "requires_mission_close_review": true
  },
  "stop": {
    "allow": false,
    "reason": "block_active_graph_wave",
    "mode": "strict",
    "message": "Codex1 says required work remains: run wave W2: T2, T3.\nContinue that now, or use $interrupt / codex1 loop pause to stop intentionally.\nIf this is a false positive, explain briefly and stop; Ralph will not block again in this turn."
  }
}
```

Stop semantics:

- No active mission: allow stop.
- Paused loop: allow stop.
- `invalid_state` or corrupt state: allow stop with warning, because Ralph must not wedge the user.
- Normal mode: block only when the status is valid, active, unpaused, and the next action is known, required, Codex-owned, and autonomous.
- Graph mode: block active, unpaused, autonomous next actions more strictly.
- No autonomous next action: allow stop and let Codex explain why it cannot
  continue autonomously.
- When Codex Stop-hook input has `stop_hook_active == true`, Ralph allows stop even if status would otherwise block.

`codex1 status` and `codex1 close check` must share readiness logic. They must not disagree about whether a durable mission is complete.

Use precise mission-close vocabulary. Do not call the mission complete when it is only ready for mission-close review.

## `codex1 doctor --json`

`doctor` proves installation and Codex integration assumptions. It is not part
of normal mission execution.

Default `doctor` should be fast and non-invasive. Deep integration probes that
spawn a custom subagent or write marker files belong behind `codex1 doctor
--json --e2e`.

Default `doctor` should check:

- `codex1` is available from a folder outside the source checkout.
- `codex1 --help` and core subcommand help are useful.
- The inline `config.toml` Ralph Stop-hook snippet parses under current Codex.
- The managed `requirements.toml` Ralph Stop-hook snippet parses under current Codex.
- The deployment exposes the Codex1 model policy: `gpt-5.5` and `gpt-5.4-mini`.
- `STATE.json.revision` and the latest `EVENTS.jsonl` revision are consistent,
  or any audit drift is reported clearly without changing mission state.
- `codex1 ralph stop-hook` emits valid Stop-hook output for allow and block.

`doctor --e2e` should additionally prove that a custom subagent role with
`[features] codex_hooks = false` does not run the Ralph Stop hook.

`doctor` must not:

- Pick fallback models.
- Rewrite mission files.
- Change `STATE.json`.
- Decide whether a mission can continue.
- Be called by Ralph as part of Stop-hook blocking.

Example output:

```json
{
  "ok": true,
  "schema_version": "codex1.doctor.v1",
  "checks": [
    {
      "id": "cli_available_outside_source",
      "ok": true
    },
    {
      "id": "inline_stop_hook_config_parses",
      "ok": true
    },
    {
      "id": "managed_stop_hook_requirements_parse",
      "ok": true
    },
    {
      "id": "model_policy_available",
      "ok": true,
      "models": ["gpt-5.5", "gpt-5.4-mini"]
    },
    {
      "id": "subagent_hooks_disabled",
      "ok": true
    },
    {
      "id": "state_event_revision_drift",
      "ok": true,
      "state_revision": 18,
      "latest_event_revision": 18
    }
  ]
}
```

## Outcome Commands

`codex1 outcome check --json`

Checks mechanical completeness:

- Required sections/fields exist.
- No fill markers remain.
- Required fields are not empty.
- Required fields are not obvious boilerplate such as `TODO`, `TBD`, or untouched template text.

It does not judge semantic quality. The main thread does.

`codex1 outcome ratify --json`

Ratifies only if `outcome check` passes.

Effects:

- Computes and stores `STATE.json.outcome.outcome_digest` from the canonical
  machine-parsed outcome content.
- Sets `STATE.json.outcome.ratified = true`.
- Sets `STATE.json.outcome.ratified_revision` to the new state revision.
- Increments `STATE.json.revision` and appends one event.

If `OUTCOME.md` changes after ratification, `codex1 status --json` must detect
the digest mismatch and project a non-terminal state rather than silently
executing against stale outcome truth.

## Plan Commands

### `codex1 plan choose-mode`

Helps the main thread select durable planning shape.

This command should support interactive and non-interactive use:

```bash
codex1 plan choose-mode
codex1 plan choose-mode --mode normal --json
codex1 plan choose-mode --mode graph --json
```

Accepted modes:

```text
normal
graph
```

Interactive prompt:

```text
Choose planning mode:
1. normal - ordinary multi-step work with checklist, acceptance, and validation
2. graph  - large/risky/multi-agent work with task IDs, dependencies, derived waves, and planned reviews
```

The CLI may record the requested mode. The CLI should not pretend to fully understand mission risk. The main thread still judges whether to escalate to graph mode.

Example:

```json
{
  "ok": true,
  "requested_mode": "normal",
  "effective_mode": "normal",
  "next_action": {
    "kind": "plan_scaffold",
    "args": ["codex1", "plan", "scaffold", "--mode", "normal"]
  }
}
```

Escalation example:

```json
{
  "ok": true,
  "requested_mode": "normal",
  "effective_mode": "graph",
  "escalation_required": true,
  "escalation_reason": "Mission touches hooks, mission-close behavior, and parallel subagent work.",
  "next_action": {
    "kind": "plan_scaffold",
    "args": ["codex1", "plan", "scaffold", "--mode", "graph"]
  }
}
```

### `codex1 plan choose-level`

Helps the main thread select planning depth before writing the plan.

This command should support interactive and non-interactive use:

```bash
codex1 plan choose-level
codex1 plan choose-level --level 1 --json
codex1 plan choose-level --level 2 --json
codex1 plan choose-level --level 3 --json
codex1 plan choose-level --level light --json
codex1 plan choose-level --level medium --json
codex1 plan choose-level --level hard --json
```

Accepted inputs:

```text
1 / light
2 / medium
3 / hard
```

Canonical stored values:

```text
light
medium
hard
```

Interactive prompt:

```text
Choose planning level:
1. light  - small/local/obvious work
2. medium - normal multi-step work
3. hard   - architecture/risky/autonomous/multi-agent work
```

Use the verbs `light`, `medium`, and `hard` in product language, stored artifacts, docs, and skill prompts. Numeric values are only CLI input aliases. Do not use `low` or `high` as planning-level names.

The CLI may ask for and record the requested level. The CLI should not pretend to fully understand mission risk. The main thread still judges whether to escalate the effective level.

Implementation rules:

- Keep interaction limited to mode/level selection.
- Do not build a long semantic questionnaire into the CLI.
- In autonomous/non-interactive contexts, require explicit flags or accept a skill-provided default.
- Record requested/effective mode and level in `PLAN.yaml`.
- Include escalation reasons only when effective mode or level is higher than requested.

### `codex1 plan scaffold`

Creates or refreshes a skeleton `PLAN.yaml` and optional directories. This should help Codex start planning without deciding the plan for it.

Examples:

```bash
codex1 plan scaffold --mode normal --level medium --json
codex1 plan scaffold --mode graph --level hard --json
```

It may create fill sections like:

```yaml
architecture:
  summary: "[codex1-fill:architecture-summary]"
```

### `codex1 plan check --json`

Checks common requirements. This command is read-only; it must not lock the plan
or mutate `STATE.json`.

- Required plan sections exist.
- `planning_mode` is `normal` or `graph`.
- `planning_level.requested` and `planning_level.effective` are valid when present.
- Acceptance criteria exist.
- Validation strategy exists.
- No fill markers remain in locked plans.

Normal-mode checks:

- Steps/checklist items have IDs or stable names.
- Each step has acceptance and proof expectations when relevant.
- No graph-only fields are required.

Graph-mode checks:

- Every task has `id`, `kind`, `depends_on`, and `spec`.
- Root tasks use `depends_on: []`.
- Dependencies exist.
- Graph has no cycle.
- Task IDs are unique.
- Review tasks reference valid targets.
- Graph/hard planning evidence exists when `planning_level.effective: hard`.

### `codex1 plan lock --json`

Locks the current durable `PLAN.yaml` as the execution route.

Examples:

```bash
codex1 plan lock --json --mission codex1-rebuild --expect-revision 4
codex1 plan lock --json --mission codex1-rebuild --replan --supersedes T4 --expect-revision 22
```

Requirements:

- `OUTCOME.md` is ratified for durable missions.
- Current `OUTCOME.md` matches `STATE.json.outcome.outcome_digest`.
- `codex1 plan check --json` passes for the selected mode.
- No fill markers remain.
- `PLAN.yaml` contains requested/effective planning mode and level when those
  are durable.
- Normal-mode plans include steps/checklist, acceptance criteria, and validation.
- Graph-mode plans include valid task IDs, `depends_on`, specs, proof strategy,
  and review requirements.

Effects:

- Computes and stores `STATE.json.plan.plan_digest`.
- Stores `STATE.json.plan.outcome_digest_at_lock` from the current ratified
  outcome digest.
- Sets `STATE.json.plan.locked = true`.
- Sets `STATE.json.plan.locked_revision` to the new state revision.
- Initializes `STATE.json.steps` or `STATE.json.tasks` from `PLAN.yaml`.
- Sets `STATE.json.planning_mode` and `STATE.json.planning_level`.
- Increments `STATE.json.revision` and appends one event.

`plan lock` is the only normal write path that transitions a durable plan from
draft/scaffolded to executable. `plan check` remains validation only.

Replan relock:

- `codex1 plan lock --replan --supersedes <ids...> --json` is the canonical
  way to apply an edited `PLAN.yaml` after replan is required.
- It requires `STATE.json.replan.required == true` unless a future explicit
  force flag is added for manual recovery.
- It validates the edited plan through the same plan checker.
- It must not reuse superseded task IDs for new work.
- It appends new task IDs, marks superseded tasks/review boundaries, refreshes
  `plan_digest` and `outcome_digest_at_lock`, clears `replan.required`, and
  appends one event.

### `codex1 plan graph --format mermaid`

Outputs the graph as Mermaid for human and agent inspection.

In normal mode it should return a clear `MODE_UNSUPPORTED` error or a simple checklist visualization. It must not imply that normal plans secretly have a graph.

### `codex1 plan waves --json`

Graph mode only.

Derives waves from `depends_on` and current state.

It does not read stored wave truth because waves are not stored.

In normal mode it should return `MODE_UNSUPPORTED` with a hint to use `codex1 task next --json` or status.

## Task Commands

`codex1 task next --json`

Returns the next ready normal step, graph task, or graph wave.

`codex1 task packet T3 --json`

Returns a worker packet the main thread can paste into a worker subagent prompt. In normal mode, step IDs such as `S2` are acceptable.

`codex1 task start T3 --json`

Transitions a task/step into progress.

`codex1 task finish T3 --proof specs/T3/PROOF.md --json`

Records proof metadata and marks the task/step ready for downstream dependencies, review, or completion.

## State And Event Safety

`STATE.json` owns current operational state.

`EVENTS.jsonl` is append-only audit history. It is not replay authority unless replay semantics are explicitly implemented and tested.

Every mutating command should:

- Read current `STATE.json`.
- Check expected revision if provided.
- Validate preconditions.
- Acquire `PLANS/<mission-id>/.codex1.lock` with an atomic create operation.
- Re-read and re-check state under the lock.
- Write state atomically through a temporary file and rename.
- Append exactly one event describing the mutation after state commit.
- Return the new state revision in JSON.

If state commits but `EVENTS.jsonl` append fails, state remains authoritative.
The command should return a warning, and `codex1 doctor --json` should report
audit drift.

Stale writers should receive a stable error:

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "REVISION_CONFLICT",
  "message": "Expected state revision 12 but current revision is 13.",
  "retryable": true
}
```

This is artifact validity, not caller identity enforcement.

## Review Commands

`codex1 review start T4 --json`

Starts a planned review task.

In normal mode, this may start an ad hoc review boundary only if the plan/status says a reviewer is needed. Do not require normal mode to manufacture review tasks.

`codex1 review packet T4 --json`

Returns a reviewer packet the main thread can paste into reviewer prompts.

`codex1 review record T4 --clean --reviewers code-reviewer,intent-reviewer --json`

Records clean review result entered by the main thread.

`codex1 review record T4 --findings-file /tmp/findings.json --json`

Records findings entered by the main thread.

`codex1 review record T4 --raw-file /tmp/raw-review.json --adjudication-file /tmp/adjudication.json --json`

Records raw reviewer output plus main-thread triage in one durable transition.
Raw findings are audit evidence; adjudication decides whether any finding
becomes current work.

Adjudication decisions:

```text
accepted_blocking
accepted_deferred
rejected
duplicate
stale
```

The CLI should persist both raw findings and adjudication. Status must only
block on current `accepted_blocking` findings.

The CLI does not know whether the caller is main thread or reviewer. The workflow and prompts govern that. Do not build caller identity checks.

Review findings should align with the official Codex review output shape:

```json
{
  "findings": [
    {
      "title": "Short finding title",
      "body": "One paragraph explaining the issue and why it matters.",
      "confidence_score": 0.82,
      "priority": 1,
      "code_location": {
        "absolute_file_path": "/abs/path/file.ts",
        "line_range": { "start": 10, "end": 12 }
      }
    }
  ],
  "overall_correctness": "patch is incorrect",
  "overall_explanation": "Blocking issue remains.",
  "overall_confidence_score": 0.77
}
```

The official Codex repo uses `confidence_score` per finding and `overall_confidence_score` for the review event. Do not invent alternate field names such as `confidence` in stored review JSON unless it is only a UI alias.

Persisted review records wrap raw reviewer output. Do not store raw reviewer
output directly as the whole durable record.

```json
{
  "schema_version": "codex1.review.v1",
  "mission_id": "codex1-rebuild",
  "review_id": "R-004",
  "boundary_id": "RB-T4-1",
  "boundary_revision": 42,
  "plan_digest": "sha256:...",
  "recorded_revision": 43,
  "raw_output": {
    "findings": [],
    "overall_correctness": "patch is correct",
    "overall_explanation": "No blocking findings.",
    "overall_confidence_score": 0.84
  },
  "adjudication": {
    "accepted_blocking_count": 0,
    "deferred_count": 0,
    "rejected_count": 0
  }
}
```

Review profiles:

| Profile | Use |
| --- | --- |
| `code_bug_correctness` | Code-producing task or code-heavy repair |
| `local_spec_intent` | One task/spec versus intended behavior |
| `integration_intent` | Multiple tasks/wave/subsystem interaction |
| `plan_quality` | Plan critique before locking, especially graph/hard plans |
| `mission_close` | Final close review |

Review record freshness:

- A review record should name the review task or mission-close boundary it applies to.
- If the target task/review boundary was superseded, the CLI should not count the record as current.
- Late records may be preserved for audit, but they must not silently change current truth.

Late-output categories:

```text
accepted_current
late_same_boundary
stale_superseded
contaminated_after_terminal
```

The CLI may classify parent-recorded review results into these categories. It must not require reviewer agents to write complex schemas directly.

## Replan Commands

`codex1 replan check --json`

Reports whether replan is required.

The detailed canonical review/repair/replan contract is in
`07-review-repair-replan-contract.md`. If this section and that file disagree,
file `07` wins.

Main reasons:

```text
repair_budget_exhausted
repeated_dirty_reviews_for_same_boundary
core_assumption_invalidated
plan_no_longer_matches_outcome
dependency_order_invalid
repair_not_converging
```

The default repair budget is two repair rounds for the same current review
boundary. After that, still dirty means autonomous replan from the locked
outcome. It does not mean `needs_user`.

`codex1 replan record --reason <code> --supersedes T4 --json`

Records that replan is required and updates `STATE.json.replan`. New tasks are
added by editing `PLAN.yaml`, not by magic.

Applying the edited plan is done by `codex1 plan lock --replan --supersedes
<ids...> --json`, so plan validation, digest refresh, task initialization, and
replan clearing stay centralized in the plan-lock path.

## Loop Commands

`codex1 loop activate --mission <id> --mode <execute|autopilot|review_loop> --json`

Sets `STATE.json.loop.active = true`, `STATE.json.loop.paused = false`, records
the loop mode, and writes or updates `PLANS/ACTIVE.json` as selector metadata.
It requires a locked plan. This command is part of the first vertical slice
because Ralph only blocks active, unpaused loops.

Loop modes:

```text
execute     = continue an already locked plan until close complete
autopilot   = continue the clarified/ratified lifecycle through plan, execute, review/repair/replan when planned, and close
review_loop = continue an explicit iterative review/fix loop
```

`manual` is represented by an inactive or paused loop; it is not a Ralph-blocking
continuous mode.

`codex1 loop pause --json`

Used by `$interrupt`.

`codex1 loop resume --json`

Used when continuing after discussion.

`codex1 loop deactivate --json`

Used when abandoning, intentionally stopping, or after terminal close.

## Ralph Hook Command

Codex hooks are stable. Ralph should use the official hook system instead of a custom wrapper runtime.

Canonical inline `config.toml` shape:

```toml
[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "codex1 ralph stop-hook"
timeout = 5
statusMessage = "checking Codex1 mission status"
```

Canonical managed `requirements.toml` shape:

```toml
[hooks]
managed_dir = "/absolute/path/to/managed/hooks"

[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "codex1 ralph stop-hook"
timeout = 5
statusMessage = "checking Codex1 mission status"
```

The implementation must include parser tests that load these exact snippets
through Codex's current config types. Do not rely on visual TOML inspection for
hook correctness.

`codex1 ralph stop-hook` behavior:

- Reads Codex Stop hook input from stdin.
- Uses `cwd` only to locate the relevant mission/root.
- May log `session_id`, `turn_id`, and `transcript_path` for diagnostics.
- If Stop hook input has `stop_hook_active == true`, fails open and allows stop.
- Must not parse `last_assistant_message` as mission truth.
- Gets the stop decision from the same logic as `codex1 status --json`.
- If `status.stop.allow == true`, exits 0 with empty stdout or `{}`.
- If `status.stop.allow == false`, verifies `status.stop.message` is non-empty,
  `next_action.owner == "codex"`, `next_action.required == true`,
  `next_action.autonomous == true`, and `next_action.kind` is on the canonical
  Ralph block allowlist.
- If those checks pass, exits 0 with
  `{"decision":"block","reason":"<status.stop.message>"}`.
- If any check fails, fails open by exiting 0 with empty stdout or `{}`.
- If status cannot be read, fails open by exiting 0 with empty stdout or a non-blocking warning.

The detailed canonical Ralph contract is in
`06-ralph-stop-hook-contract.md`. If this section and that file disagree, file
`06` wins.

The hook adapter exists only because Codex Stop hooks expect Stop-hook JSON, while `codex1 status --json` returns Codex1 status JSON.

Modern Codex hooks can also observe `PreToolUse`, `PermissionRequest`, and `PostToolUse` for `Bash`, MCP tools, and `apply_patch`. Codex also surfaces PostToolUse payloads when long-running Bash sessions complete through later polling/writes. Codex1 may use those hooks for optional append-only audit/proof capture, but they must not be required for correctness:

- Mission truth stays in `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, proofs, and review records.
- Ralph stop authority stays in `codex1 status --json`.
- Audit observers must fail open and must not mutate mission state.
- Long-running Bash session completion may be observed late; task completion still requires explicit `codex1 task finish` or normal-mode progress recording.

## Close Commands

Close commands are internal workflow commands. They are not a public user skill.

`codex1 close check --json`

Checks terminal readiness before terminalization. It does not create or mutate
`CLOSEOUT.md`.

Mission-close review is required for graph, large, risky, or explicitly
configured missions. It is not required for the first-slice simple normal
mission.

Normal-mode requirements:

- Outcome ratified if durable outcome exists.
- Plan valid for normal mode.
- Required steps complete.
- Required proof/check evidence exists.
- Required reviews, if any, are clean.
- No active blockers.

Graph/large/risky pre-close requirements:

- Outcome ratified.
- Plan valid for graph mode.
- Required non-superseded tasks complete/review-clean.
- Planned review tasks clean.
- Required proof exists.
- No active blockers.

Close check phase rules:

- If pre-close requirements fail, return `continue_required` or
  `replan_required` with the appropriate next action.
- If pre-close requirements pass and the mission does not require mission-close
  review, return `close_required` with next action `close_complete`.
- If pre-close requirements pass, mission-close review is required, and
  mission-close review has not passed, return `close_required` with next action
  `close_review`.
- If mission-close review is open, return `close_required` with next action
  `close_review`.
- If required mission-close review has passed, return `close_required` with next
  action `close_complete`.

`close check` should report whether `CLOSEOUT.md` already exists. A missing
closeout is not a failed pre-close gate; it is work for `close complete`.

`codex1 close record-review --json`

Records mission-close review result. A clean/pass result transitions close state
from `mission_close_review_open` to `mission_close_review_passed`. Dirty results
are triaged through the normal accepted-blocking repair/replan rules.

`codex1 close complete --json`

Writes or verifies `CLOSEOUT.md`, then records terminal close state, only if
`close check` passes. If a closeout already exists, it must match the current
pre-terminal state revision or be rewritten before terminal state is recorded.

Closeout revision rule:

- If current state revision is `N`, `close complete` writes/verifies
  `CLOSEOUT.md` against `pre_terminal_revision: N`.
- The command then records terminal completion at revision `N+1`.
- `STATE.json.terminal.completed_revision` is `N+1`.
- `STATE.json.close.closeout_digest` stores the digest of the closeout that was
  verified before terminalization.

## Error Shape

Use stable errors.

```json
{
  "ok": false,
  "schema_version": "codex1.error.v1",
  "code": "PLAN_INVALID",
  "message": "Graph task T3 is missing depends_on.",
  "hint": "Add depends_on: [] for root tasks or depends_on: [T...] for dependent tasks.",
  "retryable": false
}
```

Suggested codes:

```text
OUTCOME_INCOMPLETE
OUTCOME_NOT_RATIFIED
PLAN_INVALID
MODE_UNSUPPORTED
PLAN_GRAPH_CYCLE
PLAN_GRAPH_MISSING_DEP
MISSION_LOCKED
TASK_NOT_READY
MISSING_PROOF
ACCEPTED_BLOCKING_FINDINGS
REPLAN_REQUIRED
CLOSE_NOT_READY
STATE_CORRUPT
REVISION_CONFLICT
STALE_REVIEW_RECORD
TERMINAL_ALREADY_COMPLETE
```

## Verification Bar

The implementation should prove:

- `codex1 --help` explains commands.
- `codex1 doctor --json` proves install-time assumptions without changing mission state.
- `codex1 status --json` emits stable schema with `planning_mode` and `stop`.
- `codex1 plan check` accepts valid normal plans without graph fields.
- `codex1 plan lock` is the only plan command that mutates `STATE.json.plan.locked`.
- `codex1 loop activate` sets an active unpaused loop and writes `PLANS/ACTIVE.json`.
- `codex1 plan check` rejects invalid graph plans.
- `codex1 plan waves` derives graph waves and refuses normal mode clearly.
- `codex1 task packet` and `codex1 review packet` produce useful prompt packets.
- `codex1 review record` accepts official Codex-style `confidence_score` / `overall_confidence_score` fields.
- Raw review findings do not block until triaged into accepted blocking findings.
- Accepted blocking findings require repair only while the review boundary has repair budget remaining.
- Still dirty after repair budget triggers autonomous replan.
- `$interrupt` maps to `codex1 loop pause` and makes Ralph allow stop.
- `codex1 ralph stop-hook` emits valid Codex Stop-hook JSON.
- `codex1 ralph stop-hook` allows stop when Codex Stop-hook input has `stop_hook_active == true`.
- Ralph can be configured as an inline `hooks.Stop` command in `config.toml`.
- Ralph can be configured from managed `requirements.toml`.
- The exact inline and managed Ralph hook snippets parse through current Codex
  config types.
- A spawned custom subagent role with `[features] codex_hooks = false` does not
  run Ralph's Stop hook.
- Ralph does not require PreToolUse/PostToolUse state for MCP tools, `apply_patch`, or long-running Bash sessions.
- Ralph stop behavior is fail-open for missing mission, no active mission, paused loop, corrupt/invalid state, status errors, schema mismatch, unknown next action, missing stop message, and `stop_hook_active == true`.
- `codex1 close check` and `codex1 status` agree.
- The command works from outside the source folder once installed.
