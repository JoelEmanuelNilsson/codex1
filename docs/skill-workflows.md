# Skill Workflow Notes

Codex skills are the user-facing workflow. The CLI is a deterministic artifact helper.

## Native Goals

Use native `/goal` only when the user explicitly asks for a persistent objective or long-running continuation. Native Codex owns the active goal lifecycle and any goal status or usage accounting.

Codex1 skills must not create native goals for ordinary one-turn work. When a native goal is active, proofs and closeout artifacts remain durable evidence; the native goal should be completed only after the objective is genuinely achieved and the evidence has been audited.

## Setup

`codex1 setup` only materializes repo-scoped Codex1 artifact workflow guidance. It does not install hooks, manage continuation, create native goals, or report mission status.

## Clarify

Clarify is the "write me docs" style discovery skill. It gathers the user's intent, asks or resolves planning-relevant ambiguity while questions are still allowed, and preserves the understood context. It should not start execution.

Clarify may create durable notes or feed the later PRD answers, but it is not the same skill as PRD synthesis.

## Create PRD

Create PRD synthesizes everything Codex already knows from the conversation, clarification output, repo inspection, and user-provided references into `PRD.md` through `codex1 interview prd`. It should not re-interview the user by default; it should write the best PRD from available context.

## Plan

Plan reads the PRD and decides whether research is needed. For substantial uncertainty, it creates `RESEARCH_PLAN.md`, writes `RESEARCH/` records, and then writes or updates `PLAN.md`.

Plan may also create specs and ready subplans when that makes the execution route clearer. The final planning output for executable work is `EXECUTION_PROMPT.md`: a pasteable native `/goal` objective that preserves the user's explicit go moment.

The execution prompt is not a file-loading instruction. It is the objective text the user may review, edit, and manually paste into a new Codex CLI session after typing `/goal`.

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

Autonomous execution should clarify first, create the PRD, plan, ask the user to paste or modify the generated execution objective into native `/goal`, execute slices, record reviews and triage when useful, write proofs, and close out when explicit completion criteria are satisfied. It should only open a PR when PR intent is part of the PRD.
