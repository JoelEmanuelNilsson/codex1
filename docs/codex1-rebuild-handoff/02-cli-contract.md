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

The CLI is a deterministic helper. It should help Codex by making mission state queryable and writable through exact commands.

The CLI should not be an AI. It should not decide architecture, ask semantic clarification questions, spawn subagents, or detect caller identity.

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
- Emits stable JSON.
- Emits stable error codes.
- Avoids giant prose output.
- Works from any repo folder when given mission/root args or when repo root is discoverable by explicit config.
- Has no surprise interactive prompts. The exception is a command whose purpose is explicitly interactive, such as `codex1 plan choose-level`.

Mutating commands:

- Must be idempotent where possible.
- Must fail clearly when preconditions are missing.
- Should support `--dry-run` if implementation cost is small.
- Should not mutate files when validation fails.

## Minimal Command Surface

Start with this command surface.

```bash
codex1 init
codex1 status

codex1 outcome check
codex1 outcome ratify

codex1 plan choose-level
codex1 plan scaffold
codex1 plan check
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

codex1 close check
codex1 close complete
```

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
- Become a giant workflow daemon.

Role behavior is prompt-governed. Artifact shape and state transitions are CLI-governed.

## `codex1 init`

Creates:

```text
PLANS/<mission-id>/
  OUTCOME.md
  PLAN.yaml
  STATE.json
  EVENTS.jsonl
  specs/
  reviews/
```

Initial state:

```json
{
  "mission_id": "example",
  "loop": { "active": false, "paused": false, "mode": "none" },
  "tasks": {},
  "reviews": {},
  "phase": "clarify"
}
```

## `codex1 status --json`

This is the most important command.

It is consumed by:

- Skills.
- Main thread.
- Ralph.
- Humans debugging state.

It should answer:

- Is outcome ratified?
- Is plan valid?
- What phase are we in?
- Is loop active/paused?
- What is the next action?
- What tasks are ready?
- What ready wave exists?
- Is ready wave parallel-safe?
- Are review tasks ready?
- Is replan required?
- Is close ready?
- Should Ralph allow stop?

Example:

```json
{
  "ok": true,
  "mission_id": "codex1-rebuild",
  "phase": "execute",
  "loop": {
    "active": true,
    "paused": false,
    "mode": "execute"
  },
  "next_action": {
    "kind": "run_wave",
    "wave_id": "W2",
    "tasks": ["T2", "T3"]
  },
  "ready_tasks": ["T2", "T3"],
  "parallel_safe": true,
  "parallel_blockers": [],
  "review_required": [],
  "replan_required": false,
  "close_ready": false,
  "stop": {
    "allow": false,
    "reason": "active_loop",
    "message": "Run wave W2 or use $close to pause."
  }
}
```

`codex1 status` and `codex1 close check` must share readiness logic. They must not disagree about whether a mission is complete.

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

## Plan Commands

`codex1 plan choose-level`

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

Example:

```json
{
  "ok": true,
  "requested_level": "medium",
  "effective_level": "medium",
  "next_action": {
    "kind": "plan_scaffold",
    "args": ["codex1", "plan", "scaffold", "--level", "medium"]
  }
}
```

Escalation example:

```json
{
  "ok": true,
  "requested_level": "light",
  "effective_level": "hard",
  "escalation_required": true,
  "escalation_reason": "Mission touches hooks, global setup, or mission-close behavior.",
  "next_action": {
    "kind": "plan_scaffold",
    "args": ["codex1", "plan", "scaffold", "--level", "hard"]
  }
}
```

Implementation rules:

- Keep interaction limited to planning-level selection.
- Do not build a long semantic questionnaire into the CLI.
- In autonomous/non-interactive contexts, require `--level` or accept a skill-provided default.
- Record requested/effective level in `PLAN.yaml`.
- Include `escalation_reason` only when effective level is higher than requested level.

`codex1 plan scaffold --level hard --json`

Creates or refreshes a skeleton `PLAN.yaml` and spec directories. This should help Codex start planning without deciding the plan for it.

It may create fill sections like:

```yaml
architecture:
  summary: "[codex1-fill:architecture-summary]"
```

`codex1 plan check --json`

Checks:

- Required plan sections exist.
- Every task has `id`, `kind`, `depends_on`, and `spec`.
- Root tasks use `depends_on: []`.
- Dependencies exist.
- DAG has no cycle.
- Task IDs are unique.
- Review tasks reference valid targets.
- Hard planning evidence exists when `effective: hard`.

`codex1 plan graph --format mermaid`

Outputs the DAG as Mermaid for human and agent inspection.

`codex1 plan waves --json`

Derives waves from `depends_on` and current state.

It does not read stored wave truth because waves are not stored.

## Task Commands

`codex1 task next --json`

Returns the next ready task or wave.

`codex1 task packet T3 --json`

Returns a worker packet the main thread can paste into a worker subagent prompt.

`codex1 task start T3 --json`

Transitions task into progress.

`codex1 task finish T3 --proof specs/T3/PROOF.md --json`

Records proof metadata and marks task ready for downstream dependencies or review.

## Review Commands

`codex1 review start T4 --json`

Starts a planned review task.

`codex1 review packet T4 --json`

Returns a reviewer packet the main thread can paste into reviewer prompts.

`codex1 review record T4 --clean --reviewers code-reviewer,intent-reviewer --json`

Records clean review result entered by the main thread.

`codex1 review record T4 --findings-file /tmp/findings.md --json`

Records findings entered by the main thread.

The CLI does not know whether the caller is main thread or reviewer. The workflow and prompts govern that. Do not build caller identity checks.

## Replan Commands

`codex1 replan check --json`

Reports whether replan is required.

Main reason:

```text
six consecutive dirty reviews for the same active target
```

`codex1 replan record --reason <code> --supersedes T4 --json`

Records replan decision and updates state. New tasks are added by editing `PLAN.yaml`, not by magic.

## Loop Commands

`codex1 loop pause --json`

Used by `$close`.

`codex1 loop resume --json`

Used when continuing after discussion.

`codex1 loop deactivate --json`

Used when abandoning or after terminal close.

## Close Commands

`codex1 close check --json`

Checks terminal readiness.

Requires:

- Outcome ratified.
- Plan valid.
- Required non-superseded tasks complete/review-clean.
- Planned review tasks clean.
- Mission-close review clean.
- Required proof exists.
- No active blockers.

`codex1 close complete --json`

Writes terminal close state and closeout only if `close check` passes.

## Error Shape

Use stable errors.

```json
{
  "ok": false,
  "code": "PLAN_INVALID",
  "message": "Task T3 is missing depends_on.",
  "hint": "Add depends_on: [] for root tasks or depends_on: [T...] for dependent tasks.",
  "retryable": false
}
```

Suggested codes:

```text
OUTCOME_INCOMPLETE
OUTCOME_NOT_RATIFIED
PLAN_INVALID
DAG_CYCLE
DAG_MISSING_DEP
TASK_NOT_READY
PROOF_MISSING
REVIEW_FINDINGS_BLOCK
REPLAN_REQUIRED
CLOSE_NOT_READY
STATE_CORRUPT
```

## Verification Bar

The implementation should prove:

- `codex1 --help` explains commands.
- `codex1 status --json` emits stable schema.
- `codex1 plan check` rejects invalid DAGs.
- `codex1 plan waves` derives waves.
- `codex1 task packet` and `codex1 review packet` produce useful prompt packets.
- `codex1 close check` and `codex1 status` agree.
- The command works from outside the source folder once installed.
