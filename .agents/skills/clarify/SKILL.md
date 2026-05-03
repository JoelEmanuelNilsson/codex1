---
name: clarify
description: Relentlessly clarify a future Codex1 mission before PRD synthesis. Use when the user wants write-me-docs/grill-me style discovery, to stress-test an idea, resolve ambiguity, or prepare context for create-prd.
---

# Clarify

Clarify is Codex1's `grill-with-docs`: a relentless discovery session that makes the future PRD obvious, sharpens project language, and records durable decisions as they crystallize. Do not implement, plan, write `PRD.md`, or start native `/goal` unless the user explicitly switches workflows.

## Warm Start

1. Read the conversation and user-provided references.
2. Read `docs/agents/codex1-workflow.md` and `docs/agents/codex1-domain.md` if present.
3. For exact producer formats, read [CONTEXT-FORMAT.md](CONTEXT-FORMAT.md) before updating glossary docs and [ADR-FORMAT.md](ADR-FORMAT.md) before offering or writing ADRs.
4. Inspect context that can reduce questioning: existing Codex1 mission artifacts, `AGENTS.md`, `CONTEXT.md` or `CONTEXT-MAP.md`, repo ADRs, mission `ADRS/`, tests, and relevant source.
5. State the current understanding briefly when it helps the user see what you inferred.
6. Ask the highest-leverage unresolved question first.

Proceed silently if the docs or glossary do not exist. Create/update domain docs lazily only when the session resolves real language or decisions.

## Hard Rules

- Ask exactly one question at a time, waiting for feedback before continuing.
- Every question must include why it matters and your recommended answer.
- If repo inspection can answer the question, inspect first and do not ask.
- Walk each branch of the decision tree; do not settle for vibes.
- Challenge vague or overloaded words until they become concrete behavior, artifacts, or constraints.
- Cross-check claims against the code, docs, ADRs, and prior artifacts before treating them as resolved.
- Push back when the user's model conflicts with verified context.
- Keep a running decision ledger in the conversation.
- Do not create issue-tracker tickets, start implementation, write `PRD.md`, write `PLAN.md`, or create/complete native `/goal` state.

## Domain Side Effects

When a domain term is resolved, update `CONTEXT.md` inline. If `CONTEXT-MAP.md` exists, update the relevant context file. Use [CONTEXT-FORMAT.md](CONTEXT-FORMAT.md) for exact structure and rules. Do not add generic programming terms.

Offer an ADR only when all three are true:

- Hard to reverse: changing later would be meaningfully costly.
- Surprising without context: a future reader would wonder why.
- Real trade-off: plausible alternatives existed and one was chosen for a reason.

If an ADR is warranted, use [ADR-FORMAT.md](ADR-FORMAT.md). Keep it lightweight unless the decision truly needs more structure. For repo-wide decisions, prefer `docs/adr/`. For mission-specific execution decisions, prefer `.codex1/missions/<id>/ADRS/`.

## Interview Map

- Problem: what pain exists, who feels it, and what changes when solved.
- Destination: finished user/developer experience.
- Actors: human users, Codex, workers/subagents, maintainers, reviewers, CI, external systems.
- Scope: success criteria, non-goals, migration boundaries, compatibility promises, PR intent.
- UX: command flow, copy/paste moments, prompts, inspected artifacts, failure messages.
- Data and state: durable artifacts, native Codex state, receipts, logs, cache, issue trackers, and explicit non-state.
- Architecture: deep modules, interfaces, invariants, integration boundaries, ADR constraints.
- Proof: tests, commands, screenshots, manual checks, review, triage, closeout evidence.
- Completion: what makes a later `/goal` objectively done, not paused and not waiting for a question.

## Question Shape

```
Question: ...
Why it matters: ...
My recommendation: ...
```

Concrete options are useful when they clarify tradeoffs. If the user proposes a weak answer, say so and explain the failure mode.

## Stop Condition

Stop when `$create-prd` can write without guessing:

- problem and destination
- intended UX and actors
- success criteria and non-goals
- constraints and state boundaries
- domain terms and ADRs affected
- implementation decision territory
- proof and review expectations
- PR intent
- remaining assumptions acceptable to record

End with a PRD-ready clarification brief covering original request, interpreted destination, target actors, intended UX, success criteria, non-goals, constraints, verified context, domain/ADR updates, implementation decision territory, resolved questions, remaining assumptions, proof/review expectations, risks, and PR intent.

Clarification notes are inputs for `$create-prd`, not mission truth and not execution instructions.
