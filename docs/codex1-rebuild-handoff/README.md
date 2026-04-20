# Codex1 Rebuild Handoff

This folder is the handoff package for rebuilding Codex1 from scratch.

It is written for a new AI implementation agent that has no chat context and should not need to ask foundational product questions before starting. The agent should read this folder in order, then implement from it.

## Read Order

1. `01-product-flow.md`
2. `02-cli-contract.md`
3. `03-planning-artifacts.md`
4. `04-roles-models-prompts.md`
5. `05-build-prompt.md`

The older single-file architecture draft at `../codex1-rebuild-clear-architecture.md` is background. This folder is the sharper handoff contract.

## Source Inspiration

Codex1 is based on one core observation:

```text
Codex is great at writing and composing complex commands, so Codex1 should give Codex a small, composable CLI with stable JSON, predictable errors, useful help, and companion skills.
```

The implementation agent should read and follow the official OpenAI guide:

- OpenAI guide: https://developers.openai.com/codex/use-cases/agent-friendly-clis
- CLI Creator skill: https://github.com/openai/skills/tree/main/skills/.curated/cli-creator

Local installed CLI Creator paths on this machine:

- `/Users/joel/.agents/skills/cli-creator/SKILL.md`
- `/Users/joel/.claude/skills/cli-creator/SKILL.md`
- `/Users/joel/.codex/skills/cli-creator/SKILL.md`

The relevant lesson from that guide is:

- Build a command Codex can run from any folder.
- Give every command useful `--help`.
- Prefer stable JSON and small default output.
- Provide exact reads by ID and file exports for large payloads.
- Add setup/auth checks with clear failures.
- Pair the CLI with a companion skill that teaches future Codex threads which commands to run first and which writes require approval.
- Verify the installed command from outside the source folder, not only with `cargo run` or local script paths.

Codex1 should apply that lesson to Codex missions themselves.

## Product In One Paragraph

Codex1 is a skills-first native Codex workflow. Users invoke `$clarify`, `$plan`, `$execute`, `$review-loop`, `$close`, or `$autopilot`. Those skills use a small deterministic `codex1` CLI. The CLI stores visible mission files, validates a full plan with a task DAG, derives execution waves, reports next actions, records task progress, records main-thread review results, pauses/resumes the active loop, checks close readiness, and emits one status JSON for Ralph. Workers execute assigned tasks. Reviewers return findings only. The main thread records mission truth. Ralph only blocks active unpaused loops by reading `codex1 status --json`.

## Non-Negotiables

- The user-facing product is skills-first.
- The deterministic substrate is a small CLI.
- Plans are full mission plans, not just DAGs.
- Planning level is selected through `codex1 plan choose-level` using the product verbs `light`, `medium`, and `hard`; numeric input `1`, `2`, or `3` may be accepted as aliases, but `low` and `high` are not product terms.
- Every executable task has an explicit task ID and `depends_on`.
- Waves are derived from the DAG; waves are not stored as editable truth.
- Review timing is mostly represented as planned review tasks in the DAG.
- Mission-close review is mandatory.
- Workers may edit assigned paths and run proof commands.
- Reviewers return findings to the main thread and do not record review truth.
- Role boundaries are prompt-governed, not fake machine-enforced.
- The CLI must not detect "parent vs subagent."
- No `.ralph` mission state directory.
- Ralph is a tiny status guard, not an orchestrator.

## Explicit Anti-Goals

Do not build:

- A wrapper runtime around Codex.
- A giant state machine hidden in hooks.
- A fake permission system for subagents.
- Caller identity checks.
- Capability-token maze.
- Session-ID authority system.
- Reviewer writeback authority tokens.
- Stored waves as canonical truth.
- Multiple competing closeout/gate/cache truth surfaces.
- A CLI that spawns subagents.
- A CLI that asks semantic clarification questions.

## Expected Implementation Style

Build the CLI first, but remember the CLI is not the user product.

The correct split is:

```text
skills = UX and workflow guidance
CLI = deterministic state, validation, derived views, and recording
visible files = durable truth
subagents = delegated Codex reasoning/work/review
Ralph = tiny status guard
```

If a proposed implementation adds complexity, ask:

```text
Is this enforcing artifact validity, or is it pretending to enforce AI role identity?
```

Only the first is usually appropriate for the CLI.
