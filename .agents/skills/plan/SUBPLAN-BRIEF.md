# Subplan Brief Format

Ready subplans are agent briefs for future Codex work. They should stay useful even if code moves.

## Principles

- Durable over precise: describe behavior, interfaces, contracts, artifacts, and acceptance criteria.
- Behavioral over procedural: say what must be true, not which line to edit.
- Complete enough for AFK execution: a ready subplan should need no further user decision.
- Explicit scope boundaries: prevent adjacent gold-plating.
- Measurable enough to stop: name the proof, baseline, proxy, or review gate that tells an agent the slice is done.

Avoid line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, `SPECS/`, `SUBPLANS/ready/`, or a durable command.

## Slice Types

- `AFK`: an agent can execute from artifacts without more human decisions.
- `HITL`: a human decision, design review, credential, visual judgment, or manual access is still required.

Only fully specified AFK work belongs in `SUBPLANS/ready/`.

A subplan is not ready if it still needs a credential, product choice, visual judgment, environment access decision, or measurement definition. Keep that work out of `SUBPLANS/ready/`; use paused work only when a durable placeholder prevents confusion.

## Execution Lanes

Every ready subplan must include `Execution Lane` with one allowed value:

- `tdd`: behavior-changing code that should use red-green-refactor through public interfaces
- `diagnose`: hard bug or regression work that needs a reproduce-first loop
- `improve-codebase-architecture`: architecture deepening work using modules, interfaces, seams, adapters, depth, leverage, and locality
- `proof-qa`: mission-scoped acceptance proof, Browser checks, screenshots, logs, manual checks, review evidence, closeout, or accepted-risk records
- `standard`: docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial

`$plan` assigns the lane. Native `/goal` executes from the subplans.

## Template

```md
## Slice Type

AFK or HITL, with one sentence explaining why.

## Execution Lane

One of `tdd`, `diagnose`, `improve-codebase-architecture`, `proof-qa`, or `standard`.

## Current Behavior

What happens now, or what repo/artifact state currently exists.

## Desired Behavior

What should be true after this slice.

## Key Interfaces

- Stable type, command, artifact, contract, or workflow the agent should understand

## Scope

- In-scope behavior or artifact work

## Out Of Scope

- Adjacent work that should not be changed

## Dependencies

- Prior slice, spec, ADR, research, credential, or human decision required

## Blocked By

- "None" or concrete blockers

## Acceptance Criteria

- [ ] Specific, testable criterion
- [ ] Specific, testable criterion

## Measurement

- Baseline, target, proxy, checklist, eval, screenshot diff, log condition, or "not applicable" with reason

## Expected Proof

- Command, test, screenshot, manual check, log, review, or accepted-risk record

## Shortcuts To Avoid

- Metric, scope, test, workflow, or visual-reference shortcuts that would appear complete without satisfying the PRD

## Exit Criteria

- What lets Codex stop this slice with the repo working
```

## Tracer Bullet Rule

Each subplan should deliver the thinnest complete vertical path through the system that can be reviewed, tested, and proven independently.

For visual/UI slices, images are context unless the subplan names a comparison method. Prefer behavior, design-system rules, screenshots, and explicit review gates over unbounded visual perfection.
