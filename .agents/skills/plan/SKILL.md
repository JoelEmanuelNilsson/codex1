---
name: plan
description: Design an executable Codex1 mission from PRD.md, including research, specs, vertical subplans, and a native goal brief. Use after create-prd.
---

# Plan

Use this after `PRD.md` exists. Planning turns the PRD into an executable route. It is not execution and not a project-management exercise.

Completion scope default: PRD is the final finished-product contract unless it asks for staged delivery; subplans are implementation slices, not product stages.

Ask the user only when a product, scope, UX, credential, or human-judgment decision is missing. Do not ask the user to decide technical dependency ordering, slice granularity, parallelization, test placement, or other planning mechanics that Codex can infer from the repo.

Do not stop at phases, waves, or workstreams. `PLAN.md` must preserve the execution spine: outcome contract, implementation shape, execution order, ready subplans, proof strategy, risks/non-goals, and unresolved human decisions if any.

Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` if present. Read [ADR-FORMAT.md](ADR-FORMAT.md) before writing ADRs, [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md) before writing ready subplans, and [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md) before writing `GOAL_BRIEF.md`.

## Process

1. Read `PRD.md` first. Treat it as the outcome contract.
2. Inspect repo context before planning: tests, docs, domain glossary, ADRs, prior mission artifacts, and relevant code.
3. Restate the outcome contract: what must be true, what is out of scope, and what proof will matter.
4. Decide whether research is needed. If uncertainty affects architecture, product behavior, verification, or external APIs, create `RESEARCH_PLAN.md` and record research before finalizing the plan.
5. Identify the implementation shape: existing patterns, likely deep modules, needed contracts, risk areas, and whether architecture thinking is only a planning lens or a dedicated refactor mission.
6. Create ADRs in `ADRS/` when planning makes or preserves a durable architecture decision, chooses between plausible alternatives, rejects a tempting approach for a load-bearing reason, or changes a previous architectural direction. Use [ADR-FORMAT.md](ADR-FORMAT.md) and keep ADRs lightweight unless the decision needs structure.
7. Create specs for bounded contracts where implementation needs more precision than the PRD.
8. Break work into tracer-bullet vertical slices. Each slice cuts end-to-end through the smallest behavior path that can be reviewed, tested, and proven independently.
9. Assign an `Execution Lane` to every ready subplan: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial.
10. Write the execution order. Use simple serial order by default. Add parallel-safe groups only when they are obvious and useful. This is guidance, not a dependency graph engine.
11. Mark each slice as `AFK` or `HITL`. `AFK` means an agent can execute from artifacts without more human decisions. `HITL` means a human decision, design review, credential, or manual judgment is still required.
12. Put only fully specified AFK slices in `SUBPLANS/ready/`. Keep HITL work out of ready execution; use `SUBPLANS/paused/` only when a durable placeholder is useful.
13. Define proof for every executable slice: tests, commands, screenshots, logs, manual checks, review evidence, or accepted-risk records.
14. Write `GOAL_BRIEF.md` as a native goal brief that helps Codex create or refine the actual `/goal` objective.

## Artifacts

- `PLAN.md`: outcome contract, implementation shape, execution order, parallelization notes when useful, ready subplans, proof strategy, risks, and human decisions if any.
- `RESEARCH_PLAN.md`: research questions, sources, experiments, expected outputs, stopping criteria, and how findings affect the plan.
- `RESEARCH/`: durable research records with sources, facts, experiments, uncertainty, options, and recommendations.
- `ADRS/`: durable architecture decisions with context, decision, options considered, tradeoffs, consequences, and links to PRD/plan/specs.
- `SPECS/`: implementation contracts for bounded areas.
- `SUBPLANS/ready/`: executable vertical slices that require no further user decisions.
- `GOAL_BRIEF.md`: a native goal brief the user or Codex may use to create or refine the real `/goal` objective.

## Subplan Quality Bar

Every ready subplan is an agent brief. Use [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md). It must be durable even if files move, and must include:

- slice type: AFK unless already resolved HITL work has become executable
- execution lane: one of `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`
- current behavior or current repo state
- desired behavior after the slice
- key interfaces, stable types, commands, artifacts, or contracts
- exact in-scope and out-of-scope work
- dependencies and blocked-by relationships
- worker/subagent ownership rules when useful
- concrete acceptance criteria
- required proof and where to record it
- exit criteria that leave the repo working

Do not reference line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, or `SUBPLANS/ready/`. Prefer behavior and interfaces over procedural instructions.

## Goal Brief Requirements

Use [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md). The goal brief is not native goal state, not a file-loading instruction, and not a sacred final prompt. It must not say to read `GOAL_BRIEF.md` as the first execution step. It should give Codex enough context to create or refine a strong whole-mission native goal.

- mission path
- primary artifacts to read
- execution order
- subplan selection rules
- worker/subagent rules when useful
- editable scope
- proof recording rules
- review and triage rules
- explicit completion criteria
- non-completion behavior
- closeout rules
- prohibited actions

Completion criteria are only completion criteria. Do not put pause, escalation, or "ask the user" criteria under completion. The `/goal` execution phase may not ask questions. If completion cannot be reached from the artifacts, the objective should instruct Codex to record non-completion evidence, accepted risks, or deferred work instead of inventing scope or asking the user.

Do not manage native goal state from Codex1. Do not treat `codex1 setup` status or `codex1 init` output as proof of readiness or completion. The user keeps the go moment by asking Codex to create a native goal from `GOAL_BRIEF.md` or by editing the brief before `/goal`.
