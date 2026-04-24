# 10 First-Slice Skill Contracts

This file defines the minimum user-facing skill behavior for the first vertical
slice. These are product contracts for `SKILL.md` wrappers, not CLI
implementation details.

If this file disagrees with `01-product-flow.md` on first-slice skill behavior,
this file wins. If it disagrees with `02-cli-contract.md`,
`08-state-status-and-graph-contract.md`, or `09-implementation-errata.md` on
command/state details, those command/state files win.

## Shared Skill Rules

Skills are the user-facing product. Users should experience Codex1 through:

```text
$clarify
$plan
$execute
$interrupt
$autopilot
```

`$review-loop` exists as a public skill, but full planned review/replan behavior
is not part of the first slice.

Every first-slice skill should:

- Prefer chat-only work when durable state adds no value.
- Use `codex1 status --json` first when a durable mission may exist.
- Use stable CLI JSON instead of parsing prose artifacts when state matters.
- Record assumptions when proceeding autonomously.
- Avoid raw CLI ceremony in user-facing prose unless the user asks for it.
- Respect dirty worktrees and assigned write paths.
- Never overwrite user work or silently broaden ownership when the safe scope is
  unclear.

## `$clarify`

Purpose:

```text
Create a specified enough target to build the right thing.
```

First-slice behavior:

- Decide whether durable state is useful.
- If not, keep the work chat-only and hand off to ordinary Codex behavior.
- If durable state is useful, create or update `OUTCOME.md`.
- Ask only questions that change scope, risk, money, credentials, deployment,
  irreversible external actions, privacy, security, or acceptance criteria.
- Run `codex1 outcome check --json`.
- Run `codex1 outcome ratify --json` only when required fields are complete and
  the main thread judges the outcome semantically clear.

Allowed first-slice writes:

- `codex1 init --json`
- edits to `OUTCOME.md`
- `codex1 outcome ratify --json`

Must not:

- Execute implementation work.
- Lock a plan.
- Activate the loop.
- Ask semantic questions through the CLI.

## `$plan`

Purpose:

```text
Create the lightest plan that preserves intent and makes execution correctable.
```

First-slice behavior:

- Read the current request or ratified `OUTCOME.md`.
- Choose normal mode for the first durable slice unless risk requires graph mode,
  in which case stop after producing the graph-ready plan request rather than
  implementing graph machinery.
- Use `codex1 plan choose-mode --mode normal --json` and
  `codex1 plan choose-level --level <light|medium|hard> --json` when durable
  state is active.
- Run `codex1 plan scaffold --mode normal --level <level> --json`.
- Fill `PLAN.yaml` with normal steps, acceptance criteria, and validation.
- Run `codex1 plan check --json`.
- Run `codex1 plan lock --json --expect-revision <N>` only after plan check
  passes and the main thread judges the plan ready.

Allowed first-slice writes:

- `PLAN.yaml`
- `codex1 plan scaffold --json`
- `codex1 plan lock --json`

Must not:

- Execute tasks.
- Activate the loop.
- Store graph waves.
- Treat normal mode as a hidden DAG.

## `$execute`

Purpose:

```text
Execute the next ready step.
```

First-slice behavior:

- Run `codex1 status --json`.
- If a durable plan is locked and the loop is inactive, run
  `codex1 loop activate --mission <id> --mode execute --json`.
- Use `codex1 task next --json` or the status `next_action` to select the next
  normal step.
- Execute the step in the main thread or a bounded worker if useful.
- Run proportional proof commands.
- Record completion with `codex1 task finish <step-id> --proof <path-or-note>
  --json --expect-revision <N>`.
- Return to `codex1 status --json`.

Allowed first-slice writes:

- assigned implementation files
- proof artifacts when useful
- `codex1 loop activate --json`
- `codex1 task start --json`
- `codex1 task finish --json`

Must not:

- Edit outside mission scope or assigned write paths.
- Record reviews.
- Complete terminal close unless status says the next action is
  `close_complete`.

## `$interrupt`

Purpose:

```text
Pause the active loop so the user can talk without Ralph forcing continuation.
```

First-slice behavior:

- Run `codex1 loop pause --json`.
- Discuss, clarify, or answer the user normally.
- Resume only when the user or main thread intentionally returns to execution.
- Use `codex1 loop resume --json` to continue.
- Use `codex1 loop deactivate --json` only when intentionally abandoning,
  stopping, or after terminal close.

Must not:

- Mark the mission complete.
- Clear terminal state.
- Treat discussion as automatic permission to continue execution.

## `$autopilot`

Purpose:

```text
Compose clarify, plan, execute, interrupt-safe status checks, and close.
```

First-slice behavior:

- Start with `codex1 status --json` when a durable mission may exist.
- If no durable mission is needed, do ordinary Codex work with proportional
  proof and no Codex1 files.
- If outcome is missing, run the `$clarify` behavior.
- If a valid locked plan is missing, run the `$plan` behavior.
- If the durable loop should continue, run `codex1 loop activate --json`.
- Execute one status-projected autonomous next action at a time.
- Stop and explain when status returns `explain_and_stop`.
- Run `codex1 close check --json` when status projects close readiness.
- Run `codex1 close complete --json` only when the projected next action is
  `close_complete`.

Must pause for genuine user-owned decisions involving:

- scope changes
- money or account tier changes
- unavailable credentials
- deployment or irreversible external operations
- privacy or security tradeoffs not covered by the locked outcome
- non-Git-managed destructive actions
- unclear file ownership where proceeding risks overwriting user work

Must not:

- Invent user preferences that change the locked outcome.
- Ask the user to resolve ordinary implementation trouble after mission lock.
- Continue looping after `$interrupt` without an intentional resume.

## First-Slice Proof

The first slice is not done until a user can drive one durable normal mission
through skills:

```text
$clarify -> OUTCOME.md ratified
$plan -> PLAN.yaml checked and locked
$execute -> loop activated, step completed, proof recorded
$interrupt -> loop paused and Ralph allows stop
$autopilot -> resumes or continues through status
close -> codex1 close complete records terminal state
```

The proof must verify the installed `codex1` command from outside the source
folder. It must not rely only on `cargo run` or local script paths.
