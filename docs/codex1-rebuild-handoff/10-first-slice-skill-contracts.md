# 10 Foundation Skill Contracts

This file defines the minimum user-facing skill behavior for the foundation
vertical slice and the intended full-product boundary for each skill. These are
product contracts for `SKILL.md` wrappers, not CLI implementation details.

If this file disagrees with `01-product-flow.md` on foundation skill behavior,
this file wins. If it disagrees with `02-cli-contract.md`,
`08-state-status-and-graph-contract.md`, or `09-implementation-errata.md` on
command/state details, those command/state files win.

## Shared Skill Rules

Skills are the user-facing product. Users should experience Codex1 through:

```text
$clarify
$plan
$execute
$review-loop
$interrupt
$autopilot
```

The foundation slice proves the skill UX on a normal mission first. That does
not demote planned review, repair, replan, or mission-close behavior. Those are
core Codex1 product requirements, and this file fixes the skill boundaries they
must follow: `$execute` handles review boundaries already present in the locked
plan, and `$review-loop` is an explicit additional skill for iterative
review/fix loops.

Every skill should:

- Prefer chat-only work when durable state adds no value.
- Use `codex1 status --json` first when a durable mission may exist.
- Use stable CLI JSON instead of parsing prose artifacts when state matters.
- During `$clarify`, ask the questions needed to ratify outcome truth. Do not
  substitute assumptions for unresolved user-owned decisions.
- After outcome ratification and plan lock, record ordinary implementation
  assumptions when proceeding autonomously.
- Avoid raw CLI ceremony in user-facing prose unless the user asks for it.
- Respect dirty worktrees and assigned write paths.
- Never overwrite user work or silently broaden ownership when the safe scope is
  unclear.

## `$clarify`

Purpose:

```text
Create a specified enough target to build the right thing.
```

Foundation-slice behavior:

- Decide whether durable state is useful.
- If not, keep the work chat-only and hand off to ordinary Codex behavior.
- If durable state is useful, create or update `OUTCOME.md`.
- Ask only questions that change scope, risk, money, credentials, deployment,
  irreversible external actions, privacy, security, or acceptance criteria.
- Ask all such questions before ratification; do not ratify an outcome by
  silently filling user-owned decisions with assumptions.
- Run `codex1 outcome check --json`.
- Run `codex1 outcome ratify --json` only when required fields are complete and
  the main thread judges the outcome semantically clear.

Allowed foundation-slice writes:

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

Allowed foundation-slice writes:

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
Execute the locked plan end to end until close is complete.
```

Full product behavior:

- Run `codex1 status --json`.
- If a durable plan is locked and the loop is inactive, run
  `codex1 loop activate --mission <id> --mode execute --json`.
- Repeatedly use `codex1 status --json` and `codex1 task next --json` to select
  the next locked-plan action.
- Execute normal steps, graph tasks, and safe graph waves in the main thread or
  bounded workers.
- Workers produce edits and evidence; the main/root orchestrator records mission
  truth with mutating `codex1` commands.
- Run planned review boundaries when they are already part of the locked plan.
- Triage review findings through the main thread; only accepted blocking
  findings become repair work.
- Run proportional proof commands and record proof.
- Record progress with `codex1 task finish <id> --proof <path-or-note> --json
  --expect-revision <N>` or the corresponding review/close command.
- When status projects `close_complete`, run `codex1 close complete --json`.
- Stop when status returns `complete` after terminal close.

First-slice behavior:

- Prove the same continuous shape on the simple normal slice: execute all
  normal steps, record proof, run close check, run close complete, and stop at
  terminal complete.
- The foundation proof may use a normal plan with no planned review boundary,
  but the full product implementation must run planned review boundaries through
  `$execute` when they are part of the locked plan.

Allowed foundation-slice writes:

- assigned implementation files
- proof artifacts when useful
- `codex1 loop activate --json`
- `codex1 task start --json`
- `codex1 task finish --json`
- `codex1 close complete --json`

Must not:

- Edit outside mission scope or assigned write paths.
- Create, rewrite, or relock plans.
- Invent unplanned review loops.
- Open a PR.
- Complete terminal close unless status says the next action is
  `close_complete`.

## `$review-loop`

Purpose:

```text
Run an explicit iterative review/fix loop.
```

Full product behavior:

- Use when the user asks for a review loop or when the ratified outcome/locked
  plan explicitly calls for extra iterative review pressure.
- Reuse Codex1 review packet, reviewer, triage, repair, re-review, and replan
  mechanics.
- Keep looping over accepted blocking findings until clean, repair budget is
  exhausted and replan is required, or status projects `explain_and_stop`.

Foundation-slice behavior:

- The foundation proof does not need a full durable review/replan loop. The
  integrated product does: `$review-loop` must become the explicit review/fix
  loop once review recording, triage, repair, and replan are implemented.

Must not:

- Replace ordinary `$execute` behavior for planned review boundaries already in
  the locked plan.
- Treat raw reviewer findings as durable work before main-thread triage.
- Mark terminal close complete.

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
Compose clarify, plan, execute, planned review/repair/replan, and close.
```

First-slice behavior:

- Start with `codex1 status --json` when a durable mission may exist.
- If no durable mission is needed, do ordinary Codex work with proportional
  proof and no Codex1 files.
- If outcome is missing or unratified, run the `$clarify` behavior and ask the
  questions needed to ratify outcome truth.
- If a valid locked plan is missing, run the `$plan` behavior.
- If the durable loop should continue, run `codex1 loop activate --json` or
  `codex1 loop resume --json` as appropriate.
- Execute the locked plan continuously using `$execute` semantics.
- Stop and explain when status returns `explain_and_stop`.
- Run `codex1 close check --json` when status projects close readiness.
- Run `codex1 close complete --json` only when the projected next action is
  `close_complete`.
- Open a PR only when PR creation is part of the ratified outcome; otherwise
  stop at terminal close with PR-ready summary and proof.

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
- Ratify clarify-phase assumptions in place of unanswered user-owned decisions.
- Ask the user to resolve ordinary implementation trouble after mission lock.
- Continue looping after `$interrupt` without an intentional resume.
- Open a PR unless the ratified outcome says to open one.

## Foundation Proof

The foundation slice is not done until a user can drive one durable normal
mission through skills:

```text
$clarify -> OUTCOME.md ratified
$plan -> PLAN.yaml checked and locked
$execute -> loop activated, all normal steps completed, proof recorded, close complete
$interrupt -> loop paused and Ralph allows stop
$autopilot -> clarify/plan/execute/close path works through skills
```

The proof must verify the installed `codex1` command from outside the source
folder. It must not rely only on `cargo run` or local script paths.
