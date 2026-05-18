# Subplan Brief Format

Ready subplans are agent briefs for future Codex work. They should stay useful even if code moves.

## Principles

- Durable over precise: describe behavior, interfaces, contracts, artifacts, and acceptance criteria.
- Behavioral over procedural: say what must be true, not which line to edit.
- Complete enough for AFK execution: a ready subplan should need no further user decision.
- Explicit scope boundaries: prevent adjacent gold-plating.

Avoid line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, `SPECS/`, `SUBPLANS/ready/`, or a durable command.

## Slice Types

- `AFK`: an agent can execute from artifacts without more human decisions.
- `HITL`: a human decision, design review, credential, visual judgment, or manual access is still required.

Only fully specified AFK work belongs in `SUBPLANS/ready/`.

## Execution Lanes

Every ready subplan must include `Execution Lane` with one allowed value:

- `tdd`: behavior-changing code that should use red-green-refactor through public interfaces
- `diagnose`: hard bug or regression work that needs a reproduce-first loop
- `improve-codebase-architecture`: architecture deepening work using modules, interfaces, seams, adapters, depth, leverage, and locality
- `prototype`: throwaway work that answers a named design, state, or UI question
- `proof-qa`: mission-scoped acceptance proof, Browser checks, screenshots, logs, manual checks, review evidence, closeout, or accepted-risk records
- `standard`: docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial

`$plan` assigns the lane. Native `/goal` executes from the subplans.

## Template

```md
## Slice Type

AFK or HITL, with one sentence explaining why.

## Execution Lane

One of `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`.

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

## Expected Proof

- Command, test, screenshot, manual check, log, review, or accepted-risk record

## Exit Criteria

- What lets Codex stop this slice with the repo working
```

## Tracer Bullet Rule

Each subplan should deliver the thinnest complete vertical path through the system that can be reviewed, tested, and proven independently.
