# Skill Workflow Notes

Codex skills are the user-facing workflow. The CLI is a deterministic artifact helper.

## Native Goals

Use native `/goal` only when the user explicitly asks for a persistent objective or long-running continuation. Native Codex owns the active goal lifecycle and any goal status or usage accounting.

Codex1 skills must not create native goals for ordinary one-turn work. When a native goal is active, proofs and closeout artifacts remain durable evidence; the native goal should be completed only after the objective is genuinely achieved and the evidence has been audited.

## Setup

`codex1 setup` materializes repo-scoped Codex1 artifact workflow guidance: a small overview skill, core workflow skills (`$clarify`, `$create-prd`, `$plan`), and repo-local lane skills (`$tdd`, `$diagnose`, `$improve-codebase-architecture`, `$prototype`). It does not install hooks, manage continuation, create native goals, or report mission status.

## Clarify

Clarify is the Codex1 discovery skill. It gathers the user's intent, asks or resolves planning-relevant ambiguity while questions are still allowed, and preserves the understood context. It asks one question at a time, explains why the question matters, recommends an answer, inspects the repo instead of asking code-answerable questions, challenges vague terms until they become concrete behavior, updates `CONTEXT.md` lazily when language crystallizes, and offers ADRs only for durable, surprising tradeoffs. It should not start execution.

Clarify may create durable notes or feed the later PRD answers, but it is not the same skill as PRD synthesis.

## Create PRD

Create PRD synthesizes everything Codex already knows from the conversation, clarification output, repo inspection, and user-provided references into `PRD.md` through `codex1 interview prd`. It should not re-interview the user by default; it should write the best PRD from available context.

The PRD should carry enough product and implementation context for `$plan`: problem statement, solution, extensive user stories, module sketch, implementation decisions, testing decisions, out-of-scope work, proof expectations, review expectations, and PR intent. It stays inside the Codex1 mission artifact tree.

## Plan

Plan reads the PRD and turns it into an executable route. For substantial uncertainty, it creates `RESEARCH_PLAN.md`, writes `RESEARCH/` records, and then writes or updates `PLAN.md`.

Plan may also create ADRs, specs, and ready subplans when that makes the execution route clearer. ADRs are for durable architecture decisions and rejected alternatives with load-bearing reasons, not tiny implementation notes. Architecture work can be a planning lens or its own refactor mission. Ready subplans should be tracer-bullet vertical slices and durable agent briefs with current/desired behavior, key interfaces, scope, out-of-scope work, dependencies, acceptance criteria, proof, ownership rules, and exit criteria. The planner decides technical ordering and parallel-safe work; ask the user only for missing product, scope, UX, credential, or human-judgment decisions.

`PLAN.md` should not stop at phases, waves, or workstreams. It should preserve the execution spine: outcome contract, implementation shape, execution order, ready subplans, proof strategy, risks/non-goals, and unresolved human decisions if any.

The final planning output for executable work is `GOAL_BRIEF.md`: a native goal brief that preserves the user's explicit go moment without pretending Codex1 owns the goal.

The goal brief is not native goal state, not a file-loading instruction, and not a sacred final prompt. It should give Codex enough context to create or refine a whole-mission `/goal`, and it must not tell Codex to read `GOAL_BRIEF.md` as the first execution step.

In Plan mode, native goal continuation is suppressed by Codex itself. Codex1 should still only write artifacts the user requested or the plan clearly needs.

## Execute

Execute works from active or ready subplans. Each executable slice should have a subplan. Workers receive the PRD, plan, relevant spec, relevant subplan, applicable ADRs, explicit ownership, proof expectations, and non-goals.

Workers should not edit mission-level artifacts unless explicitly assigned. If implementation reveals a mismatch, they should report it or update only their assigned spec when allowed.

## Review Cycle

Reviewers record opinions through `codex1 interview review`. Main Codex records adjudication through `codex1 interview triage`.

Review artifacts are opinion records. Triage is main-Codex judgment. Neither is a CLI gate.

## Proof And Closeout

After a subplan is completed, Codex writes a proof artifact. When Codex judges the PRD is satisfied, it writes closeout.

Closeout summarizes the real state, including completed, superseded, paused, and deferred work. Closeout does not complete a native goal by itself.

## Interrupt And Resume

Interrupt and resume behavior belongs to native Codex, not Codex1. If a persistent objective should continue after interruption, use the official goal UI or goal tools. Do not create Codex1 files to simulate continuation.

Autonomous execution should clarify first, create the PRD, plan, create or refine the native `/goal` from `GOAL_BRIEF.md`, execute slices, record reviews and triage when useful, write proofs, and close out when explicit completion criteria are satisfied. It should only open a PR when PR intent is part of the PRD.
