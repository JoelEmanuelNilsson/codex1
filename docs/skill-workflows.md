# Skill Workflow Notes

Codex skills are the user-facing workflow. The CLI is only setup and path-safe mission scaffolding.

## Native Goals

Use native `/goal` only when the user explicitly asks for a persistent objective or long-running continuation. Native Codex owns the active goal lifecycle and any goal status or usage accounting.

Codex1 skills must not create native goals for ordinary one-turn work. When a native goal is active, proofs and closeout artifacts remain durable evidence; the native goal should be completed only after the objective is genuinely achieved and the evidence has been audited.

## Setup

`codex1 setup` materializes repo-scoped Codex1 artifact workflow guidance: core workflow skills (`$clarify`, `$create-prd`), repo-local lane skills (`$tdd`, `$diagnose`, `$improve-codebase-architecture`), the `$codex-review` closeout helper, and the `$handoff` continuation helper. `codex1 init` creates the standard mission directory tree. The CLI does not manage continuation, create native goals, report mission status, or write semantic artifacts.

## Clarify

Clarify is the Codex1 discovery skill. It gathers the user's intent, asks or resolves planning-relevant ambiguity while questions are still allowed, and preserves the understood context. It asks one question at a time, explains why the question matters, recommends an answer, inspects the repo instead of asking code-answerable questions, challenges vague terms until they become concrete behavior, updates `CONTEXT.md` lazily when language crystallizes, and offers ADRs only for durable, surprising tradeoffs. It should not start execution.

Clarify may create durable notes or feed the later PRD answers, but it is not the same skill as PRD synthesis.

## Create PRD

Create PRD synthesizes everything Codex already knows from the conversation, clarification output, repo inspection, and user-provided references into `PRD.md`. It should not re-interview the user by default; it should write the best PRD from available context.

The PRD should carry enough product and implementation context for direct execution or a native `/goal`: problem statement, solution, extensive behavior-focused user stories, observable success criteria, boundaries, module sketch, implementation decisions, testing decisions, proof expectations, review expectations, and PR intent. Boundaries should distinguish what must always be preserved, what requires approval before changing, and what is out of scope. The PRD stays inside the Codex1 mission artifact tree.

## Execute

Execute works from the PRD, current user request, current repo evidence, and any optional specs or subplans that already exist. Subplans are helpful only when separate execution slices genuinely reduce ambiguity. Workers receive the PRD, relevant specs or subplans, applicable ADRs, explicit ownership, proof expectations, and non-goals.

Workers should not edit mission-level artifacts unless explicitly assigned. If implementation reveals a mismatch, they should report it or update only their assigned spec when allowed.

## Review Cycle

Reviewers record opinions in `REVIEWS/`. Main Codex records adjudication in `TRIAGE/`.

Review artifacts are opinion records. Triage is main-Codex judgment. Neither is a CLI gate.

Use `$codex-review` when a mission needs Codex's built-in reviewer to inspect a dirty diff, branch, or commit before closeout. Treat the review result as advisory evidence: verify findings against the real code path, fix accepted findings, rerun focused tests and review after review-triggered edits, and record useful review/triage artifacts when the mission asks for them.

## Proof And Closeout

After a subplan is completed, Codex writes a proof artifact. When Codex judges the PRD is satisfied, it writes closeout.

Closeout summarizes the real state, including completed, superseded, paused, and deferred work. Closeout does not complete a native goal by itself.

## Interrupt And Resume

Interrupt and resume behavior belongs to native Codex, not Codex1. If a persistent objective should continue after interruption, use the official goal UI or goal tools. Do not create Codex1 files to simulate continuation.

Use `$handoff` when a human wants a compact temporary note for another agent or future fresh context. Handoffs should reference existing artifacts instead of duplicating them, live outside the repo by default, and not be treated as mission state.

Autonomous execution for a PRD-backed mission should clarify first, create the PRD, create or refine a native `/goal` only when the user wants persistence, execute with the right lane skill when useful, record reviews and triage when useful, write proofs, and close out when explicit completion criteria are satisfied. It should only open a PR when PR intent is part of the PRD.
