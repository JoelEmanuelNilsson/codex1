# Codex1 Rebuild Handoff

This folder is the handoff package for rebuilding Codex1 from scratch.

It is written for a new AI implementation agent that has no chat context and should not need to ask foundational product questions before starting. The agent should read this folder in order, then implement from it.

## Read Order

1. `00-why-and-lessons.md`
2. `01-product-flow.md`
3. `02-cli-contract.md`
4. `03-planning-artifacts.md`
5. `04-roles-models-prompts.md`
6. `05-build-prompt.md`
7. `06-ralph-stop-hook-contract.md`
8. `07-review-repair-replan-contract.md`
9. `08-state-status-and-graph-contract.md`
10. `09-implementation-errata.md`
11. `10-first-slice-skill-contracts.md`

The older single-file architecture draft at `../codex1-rebuild-clear-architecture.md` is background. This folder is the sharper handoff contract.

If this folder disagrees with older V2 docs, this folder wins. Older docs are useful inspiration, but they contain machinery this rebuild explicitly avoids.

If files `00` through `05` disagree with files `06` through `09` on Ralph,
review loops, model choices, post-lock autonomy, state revisions, status
verdicts, or graph/wave derivation, files `06` through `09` win. They capture
the later decisions made after the initial handoff draft. File `09` adds
implementation exactness for Codex hook config, doctor checks, and foundation
build order.

If `01-product-flow.md` disagrees with `10-first-slice-skill-contracts.md` on
skill boundary behavior or foundation skill wrapper behavior, file `10` wins.
Command and state details still live in `02`, `08`, and `09`.

Files named `REVIEW-*` are critique artifacts, not canonical product direction.
They may contain rejected proposals. Use this read order and the explicit
precedence notes above for implementation truth.

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

Codex1 is a way to make Codex much more powerful while keeping the user
experience native to Codex. The user-facing product is skills: users invoke
`$clarify`, `$plan`, `$execute`, `$review-loop`, `$interrupt`, or `$autopilot`.
Those skills use a small deterministic `codex1` CLI when durable mission state,
gates, recovery, or Ralph stop pressure are useful. `$execute` runs an already
locked plan end to end until terminal close is complete, including planned
review boundaries that are part of that locked plan. `$autopilot` owns the
larger lifecycle: clarify, plan, execute, review/repair/replan when planned or
ratified, and close; it may open a PR only when the clarified outcome explicitly
asks for that. `$review-loop` is an additional explicit skill for iterative
review/fix loops, not the default meaning of every planned review boundary. The
workflow has two planning modes: normal work uses lightweight planning that can
be chat-only or durable, and large/risky work uses graph planning with task IDs,
dependencies, derived waves, planned review gates, and stricter status. Workers
execute bounded assignments. Reviewers return findings only. One explorer role
gathers facts when needed. The main thread owns user intent, synthesis, mission
truth, and completion. Ralph is only a minimal stop guard over
`codex1 status --json`; it must never become an orchestrator.

## Layer Ownership

| Layer | Owns | Must Not Own |
| --- | --- | --- |
| Skills | User-facing workflow, interviewing, planning posture, orchestration instructions | Hidden state, stale truth derivation, gate math |
| CLI | Validation, status projection, normal/graph plan checks, graph wave derivation, task/review/close state | Architecture choice, user intent, AI role identity |
| Visible files | Outcome, plan, state, audit, specs, proofs, reviews, closeout when durable state is needed | Chat-only truth for work that does not need durable state |
| Subagents | Bounded exploration, writing, reviewing, critique | Mission truth, mission close, hidden state |
| Ralph | Stop guard over `codex1 status --json` | Planning, reviewing, executing, subagent management |

## Non-Negotiables

- The user-facing product is skills-first.
- Codex1 is an autonomous Codex power-up, not a separate wrapper runtime.
- The deterministic substrate is a small CLI.
- Skills are the UX; the CLI exists so skills and Codex can rely on exact
  durable state when autonomy needs it.
- `codex1 status --json` is the first-class product artifact for skills,
  Ralph, humans, and tests.
- Ralph should be installed through Codex's stable hook system as a `Stop` hook.
- Planning is adaptive, not always heavy.
- Normal work uses lightweight planning with acceptance criteria and validation; it may stay chat-only when durable state adds no value.
- Ralph should not block unless there is an active unpaused mission with an autonomous next action.
- Ralph blocks at most once per Codex Stop-hook continuation cycle; when `stop_hook_active == true`, Ralph allows stop.
- Large/risky/multi-agent work uses graph planning.
- Graph-mode tasks have explicit task IDs and `depends_on`; normal-mode steps do not need to pretend to be a DAG.
- Graph waves are derived from dependencies and current state; waves are not stored as editable truth.
- Review timing is risk-scaled: lightweight self-review for normal work, planned review tasks and mission-close review for graph/large/risky work.
- Review findings are observations, not work; only accepted blocking findings can block progress.
- `$execute` is continuous inside the locked plan. It should stop only when
  terminal close is complete, the loop is interrupted, or status projects a
  non-autonomous `explain_and_stop`.
- `$execute` runs planned review boundaries that already exist in the locked
  plan; `$review-loop` is reserved for explicitly requested iterative review
  and fix cycles beyond ordinary execution.
- `$autopilot` follows `$clarify` before planning. It must ask the questions
  needed to ratify outcome truth rather than silently replacing them with
  assumptions.
- `$autopilot` does not open a PR by default. It may open one only when PR
  creation is part of the ratified outcome; otherwise it stops at close-complete
  or PR-ready state.
- After mission lock, ordinary ambiguity and dirty review loops should resolve through assumption recording, repair, or autonomous replan, not user questions.
- Do not use `needs_user`, `blocked_external`, or `validation_required` as normal post-lock execution verdicts.
- `$interrupt` is the user-facing discussion boundary; it pauses active loops so the user can talk without Ralph forcing continuation.
- Terminal completion is internal CLI/workflow state, not a public `$finish` or `$complete` skill.
- Workers may edit assigned paths and run proof commands.
- Reviewers return findings to the main thread and do not record review truth.
- Reviewer findings should follow official Codex review shape: `priority`, per-finding `confidence_score`, and `overall_confidence_score`.
- Custom subagent roles must disable Codex hooks with `[features] codex_hooks = false`; only the main/root orchestrator should feel Ralph stop pressure.
- Do not use full-history forks for Codex1 custom-role subagents; use explicit task packets.
- Role boundaries are prompt-governed, not fake machine-enforced.
- The CLI must not detect "parent vs subagent."
- No `.ralph` mission state directory.
- Ralph is a minimal status guard, not an orchestrator.
- Pre/Post tool hooks can observe MCP tools, `apply_patch`, and Bash, but they must not become mission truth or Ralph's control plane.

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
- A universal DAG requirement for every task.
- A public `$finish` or `$complete` skill.
- A custom wrapper runtime for stop handling now that Codex has stable hooks.
- A Ralph design that depends on observing every tool call.
- Multiple competing closeout/gate/cache truth surfaces.
- A CLI that spawns subagents.
- A CLI that asks semantic clarification questions.

## Expected Implementation Style

Build the CLI first, but remember the CLI is not the user product.

Codex1 is one integrated product. Normal planning, graph planning, planned
review boundaries, repair, replan, mission-close review, Ralph, status, and the
skills are not separate products and not optional "future" extensions.

Build the foundation vertical slice in `09-implementation-errata.md` first
because it proves the state store, status projector, loop activation, close
path, and Ralph on the smallest useful route. That is implementation order, not
product scope. The product is not complete until the graph/review/repair/replan
and mission-close contracts in this handoff also work.

Do not start by building every command module in parallel, but do not interpret
the normal slice as an MVP that replaces the full Codex1 product.

The correct split is:

```text
skills = UX and workflow guidance
CLI = deterministic state, validation, derived views, and recording
visible files = durable truth when durable truth is useful
subagents = delegated Codex reasoning/work/review
Ralph = minimal status guard
```

If a proposed implementation adds complexity, ask:

```text
Is this preserving user intent through uncertainty, or is it pretending to enforce AI role identity?
```

Only the first is usually appropriate for the CLI.
