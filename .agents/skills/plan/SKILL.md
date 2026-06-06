---
name: plan
description: "Create the lean Codex1 plan pack from PRD.md for PRD-backed product missions: PLAN.md, GOAL_BRIEF.md, and only useful ready subplans. Do not use for diagnosis, optimization research, benchmarking, review, prompt/goal prep, or execution."
---

# Plan

Use this only after `PRD.md` exists for a product mission that needs durable execution artifacts. Planning turns the PRD into an executable route and a native `/goal`-ready handoff. It is not execution, project management, status tracking, or generic goal writing.

Completion scope default: PRD is the final finished-product contract unless it asks for staged delivery; subplans are implementation slices, not product stages.

Ask the user only when a product, scope, UX, credential, or human-judgment decision is missing. Do not ask the user to decide technical dependency ordering, slice granularity, parallelization, test placement, measurement shape, or other planning mechanics that Codex can infer from the PRD and repo.

Do not stop at phases, waves, or workstreams. `PLAN.md` must preserve the execution spine: outcome contract, implementation shape, execution order, ready subplans, proof strategy, risks/non-goals, and unresolved human decisions if any.

Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` if present. Read [ADR-FORMAT.md](ADR-FORMAT.md) before writing ADRs, [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md) before writing ready subplans, and [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md) before writing `GOAL_BRIEF.md`.

## Suitability Gate

Before writing artifacts, decide whether `$plan` is the right workflow.

Use `$plan` when:

- A PRD exists and should become a whole-mission execution route.
- The work needs durable ordering, proof, closeout, and maybe multiple executable slices.
- A future native `/goal` or worker session will benefit from reading mission artifacts.

Do not use `$plan` when the user asks to diagnose, debug, optimize, investigate, benchmark, review, prepare a prompt, prepare a goal, or explicitly says not to use `$plan`. In those cases, use the relevant lane skill or direct workflow, and write only the requested docs or copy-paste goal prompt.

If the request is only "make a `/goal` prompt from known context", write a compact goal prompt directly. Do not create a Codex1 plan pack.

## Goal-Mode Defaults

Plan for native `/goal` as a long-running execution loop with clear exit criteria. Resolve these defaults yourself unless they require a human decision:

- Completion: PRD success criteria become verifiable `/goal` exit criteria. Prefer numbers, thresholds, parity targets, pass/fail checks, or named proof artifacts over vague quality claims.
- Guidance: include known starting points, useful tools, likely risk areas, and wrong-path constraints so execution does not rediscover obvious context.
- Measurement: define baseline, target, proxy, eval, screenshot diff, log, command, or manual check for each meaningful progress dimension. If no measurement exists, either plan a small measurement-building slice or mark the work HITL/blocked.
- Environment: name the most realistic environment the mission needs, such as local, Browser, preview deploy, staging, production-like data, or external device. Record mismatches as risks or stop/report rules.
- Visual goals: treat images as context. Completion should rely on feature checklists, specs, design-system adherence, screenshot diffs, or explicit visual review gates, not a vague "pixel perfect" claim.
- Anti-gaming: prohibit satisfying metrics by shrinking scope, removing coverage, inlining reference images, bypassing workflows, or weakening user-visible behavior.
- Handoff: the future `/goal` must know how to choose the next action, record progress, preserve constraints, report blockers, clean up failed attempts, and run closeout review.

## Artifact Minimalism

The default output is the smallest executable spine:

- `PLAN.md`
- `GOAL_BRIEF.md`
- `SUBPLANS/ready/` only when separate slices will actually guide execution

Create optional artifacts only when they have a named consumer:

- `RESEARCH_PLAN.md` and `RESEARCH/` only when uncertainty changes architecture, product behavior, verification, or external API usage.
- `ADRS/` only for durable, surprising, hard-to-reverse decisions with real alternatives.
- `SPECS/` only for bounded contracts that implementation agents need more precisely than the PRD and plan.
- `SUBPLANS/paused/` only for durable HITL placeholders that prevent future confusion.

Do not create an artifact merely because the old workflow listed it. Empty or generic artifacts make the next agent dumber.

## Process

1. Read `PRD.md` first. Treat it as the outcome contract.
2. Apply the Suitability Gate. If `$plan` is wrong for the request, say so briefly and use the right workflow instead.
3. Inspect repo context before planning: tests, docs, domain glossary, ADRs, prior mission artifacts, and relevant code.
4. Restate the outcome contract: what must be true, what is out of scope, how progress will be measured, and what proof will matter.
5. Identify the implementation shape: existing patterns, likely deep modules, needed contracts, risk areas, realistic environment, and whether architecture thinking is only a planning lens or a dedicated refactor mission.
6. Convert any vague or visual success criteria into observable checks, explicit review gates, or unresolved human decisions.
7. Decide which optional artifacts are genuinely needed, using Artifact Minimalism.
8. Create research artifacts, ADRs, and specs only when their conditions are met.
9. Break work into tracer-bullet vertical slices only when multiple independent execution contracts are useful. Each slice cuts end-to-end through the smallest behavior path that can be reviewed, tested, and proven independently.
10. Assign an `Execution Lane` to every ready subplan: `tdd`, `diagnose`, `improve-codebase-architecture`, `proof-qa`, or `standard`. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial.
11. Write the execution order. Use simple serial order by default. Add parallel-safe groups only when they are obvious and useful. This is guidance, not a dependency graph engine.
12. Mark each slice as `AFK` or `HITL`. `AFK` means an agent can execute from artifacts without more human decisions. `HITL` means a human decision, design review, credential, or manual judgment is still required.
13. Put only fully specified AFK slices in `SUBPLANS/ready/`. Keep HITL work out of ready execution; use `SUBPLANS/paused/` only when a durable placeholder is useful.
14. Define proof for every executable slice: tests, commands, screenshots, logs, manual checks, review evidence, accepted-risk records, and metrics or baselines when measurable.
15. Run a lightweight execution-readiness audit only for capabilities this mission actually needs. Classify each relevant capability as `proven`, `safe during goal`, `needs user decision`, or `blocked`; encode user-decision and blocked cases as stop/report rules.
16. Shape the native goal contract: desired end state, specific evidence, preserved constraints, next-action policy between continuations, progress tracking rule, blocked-report behavior, cleanup/review rule, and prohibited shortcuts.
17. Write `PLAN.md` as the lean route map and `GOAL_BRIEF.md` as the rich native goal brief.
18. Run the Final Audit below before stopping.

## Artifacts

- `PLAN.md`: required. Outcome contract, implementation shape, execution order, useful parallelization notes, ready subplans if any, proof strategy, metrics/environment assumptions, risks, and human decisions if any.
- `GOAL_BRIEF.md`: required. Rich native goal brief the user or Codex may use to create or refine the real `/goal` objective. It must be enough to start execution without a planning replay.
- `SUBPLANS/ready/`: conditional. Executable vertical slices that require no further user decisions.
- `RESEARCH_PLAN.md`: conditional. Research questions, sources, experiments, expected outputs, stopping criteria, and how findings affect the plan.
- `RESEARCH/`: conditional. Durable research records with sources, facts, experiments, uncertainty, options, and recommendations.
- `ADRS/`: conditional. Durable architecture decisions with context, decision, options considered, tradeoffs, consequences, and links to PRD/plan/specs.
- `SPECS/`: conditional. Implementation contracts for bounded areas.

## Subplan Quality Bar

Every ready subplan is an agent brief. Use [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md). Keep it compact enough to be read and used. Aim for 20-40 lines unless the slice is unusually risky. It must be durable even if files move, and must include:

- slice type: AFK unless already resolved HITL work has become executable
- execution lane: one of `tdd`, `diagnose`, `improve-codebase-architecture`, `proof-qa`, or `standard`
- current behavior or current repo state
- desired behavior after the slice
- key interfaces, stable types, commands, artifacts, or contracts
- exact in-scope and out-of-scope work
- dependencies and blocked-by relationships
- worker/subagent ownership rules when useful
- concrete acceptance criteria
- required proof and where to record it
- progress measurement, baseline, or proxy when the slice meaningfully changes quality, performance, UX, or reliability
- shortcuts to avoid when a metric or visual target could be gamed
- exit criteria that leave the repo working

Do not reference line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, or `SUBPLANS/ready/`. Prefer behavior and interfaces over procedural instructions.

## Goal Brief Requirements

Use [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md). The goal brief is not native goal state, not a file-loading instruction, not a sacred final prompt, and not automatically character-limited. It must not say to read `GOAL_BRIEF.md` as the first execution step. It should give Codex enough context to create or refine a strong whole-mission native goal with clear exit criteria.

If the user asks for the exact pasteable `/goal` prompt, write a compact `Suggested Goal Request` inside `GOAL_BRIEF.md` or create `GOAL_PROMPT.md` when that is clearer. Keep the pasteable prompt under the requested limit; do not compress the whole rich brief just to satisfy a prompt limit.

- mission path
- primary artifacts to read
- execution order
- subplan selection rules
- worker/subagent rules when useful
- editable scope
- proof recording rules
- review and triage rules
- metrics, baselines or proxies, validation loop, anti-gaming constraints, realistic environment, and readiness facts
- iteration policy for how Codex chooses the next best action between continuations
- `notes.md` tracking rule for long-running decisions, measurements, blockers, and next steps
- progress handoff rules such as meaningful commits, draft PR, progress artifact, or status posts only when useful for this mission
- explicit completion criteria
- stop and ask rules before execution; stop and report rules during execution
- non-completion behavior
- closeout rules, including review, proof audit, and cleanup of failed or superseded attempts
- prohibited actions

Completion criteria are only completion criteria. Do not put pause, escalation, or "ask the user" criteria under completion. The `/goal` execution phase may not ask questions. If completion cannot be reached from the artifacts, the objective should instruct Codex to record non-completion evidence, accepted risks, or deferred work instead of inventing scope or asking the user.

Do not manage native goal state from Codex1. Do not treat `codex1 setup` status or `codex1 init` output as proof of readiness or completion. The user keeps the go moment by asking Codex to create a native goal from `GOAL_BRIEF.md` or by editing the brief before `/goal`.

## Final Audit

Before final response, check:

- `$plan` was suitable; otherwise no plan pack was created.
- The artifact set is minimal and every optional artifact has a named consumer.
- `PLAN.md` preserves outcome, order, proof, environment, risks, and human decisions.
- Every ready subplan is AFK, lane-assigned, independently provable, and guarded against obvious metric/visual shortcuts.
- `GOAL_BRIEF.md` gives native `/goal` measurable exit criteria, next-action policy, tracking, blocked-report behavior, closeout rules, and prohibited actions without managing goal state.
