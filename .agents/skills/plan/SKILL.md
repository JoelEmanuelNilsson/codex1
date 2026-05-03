---
name: plan
description: Design an executable Codex1 mission from PRD.md, including research, specs, vertical subplans, and the pasteable native /goal objective. Use after create-prd; do not create issue-tracker tickets.
---

# Plan

Use this after `PRD.md` exists. Planning designs the mission; it is not execution. Questions are allowed during planning when they improve the plan, but the generated `/goal` objective must not ask questions.

Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` if present.

## Process

1. Read `PRD.md` first. Treat it as the outcome contract.
2. Inspect repo context before planning: tests, docs, domain glossary, ADRs, prior mission artifacts, and relevant code.
3. Restate the outcome contract: success criteria, non-goals, proof expectations, review expectations, PR intent, and assumptions.
4. Decide whether research is needed. If uncertainty affects architecture, product behavior, verification, or external APIs, create `RESEARCH_PLAN.md` and record research before finalizing the plan.
5. Identify workstreams, risks, dependencies, existing patterns, and likely deep modules.
6. Create ADRs in `ADRS/` when planning makes or preserves a durable architecture decision, chooses between plausible alternatives, rejects a tempting approach for a load-bearing reason, or changes a previous architectural direction. Keep ADRs lightweight unless the decision needs structure.
7. Create specs for bounded contracts where implementation needs more precision than the PRD.
8. Break work into tracer-bullet vertical slices. Each slice cuts end-to-end through the smallest behavior path that can be reviewed, tested, and proven independently.
9. Mark each slice as `AFK` or `HITL`. `AFK` means an agent can execute from artifacts without more human decisions. `HITL` means a human decision, design review, credential, or manual judgment is still required.
10. Quiz the user on the proposed breakdown when practical: granularity, dependency relationships, HITL/AFK labels, merge/split choices, and user stories covered. Iterate if they answer. If the user is absent, record assumptions and continue.
11. Put only fully specified AFK slices in `SUBPLANS/ready/`. Keep HITL work in `PLAN.md` or move it to `SUBPLANS/paused/` if it needs a durable placeholder.
12. Define proof for every executable slice: tests, commands, screenshots, logs, manual checks, review evidence, or accepted-risk records.
13. Write `EXECUTION_PROMPT.md` with a pasteable native `/goal` objective.

## Artifacts

- `PLAN.md`: strategy thesis, workstreams, phases, risk map, artifact index, review posture, and recommended next slices.
- `RESEARCH_PLAN.md`: research questions, sources, experiments, expected outputs, stopping criteria, and how findings affect the plan.
- `RESEARCH/`: durable research records with sources, facts, experiments, uncertainty, options, and recommendations.
- `ADRS/`: durable architecture decisions with context, decision, options considered, tradeoffs, consequences, and links to PRD/plan/specs.
- `SPECS/`: implementation contracts for bounded areas.
- `SUBPLANS/ready/`: executable vertical slices that require no further user decisions.
- `EXECUTION_PROMPT.md`: the objective the user may review, edit, and paste after `/goal`.

## Subplan Quality Bar

Every ready subplan is an agent brief. It must be durable even if files move, and must include:

- slice type: AFK unless already resolved HITL work has become executable
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

## Execution Objective Requirements

The goal prompt is the pasteable objective text, not a file-loading instruction and not a wrapper around another prompt. It must not say to read `EXECUTION_PROMPT.md`; the user is copying from that file into `/goal`.

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

Do not create issue-tracker tickets. Do not create, inspect, or complete native goal state. Do not treat Codex1 inspect/status/events/receipts as proof of readiness or completion. The user keeps the go moment by manually starting a new Codex CLI session, typing `/goal`, and pasting the generated objective.
