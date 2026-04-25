# Skill Workflow Notes

Codex skills are the user-facing workflow. The CLI is a deterministic artifact helper.

## Clarify

Clarify gathers enough user intent to write `PRD.md` through `codex1 interview prd`. Codex decides how much detail the mission needs.

## Plan

Plan reads the PRD and decides whether research is needed. For substantial uncertainty, it creates `RESEARCH_PLAN.md`, writes `RESEARCH/` records, and then writes or updates `PLAN.md`.

Plan may also create specs and ready subplans when that makes the execution route clearer.

## Execute

Execute works from active or ready subplans. Each executable slice should have a subplan. Workers receive the PRD, plan, relevant spec, relevant subplan, applicable ADRs, explicit ownership, proof expectations, and non-goals.

Workers should not edit mission-level artifacts unless explicitly assigned. If implementation reveals a mismatch, they should report it or update only their assigned spec when allowed.

## Review Loop

Reviewers record opinions through `codex1 interview review`. Main Codex records adjudication through `codex1 interview triage`.

Review artifacts are opinion records. Triage is main-Codex judgment. Neither is a CLI gate.

## Proof And Closeout

After a subplan is completed, Codex writes a proof artifact. When Codex judges the PRD is satisfied, it writes closeout.

Closeout summarizes the real state, including completed, superseded, paused, and deferred work.

## Interrupt And Autopilot

Interrupt should pause explicit loop state:

```sh
codex1 --mission <id> loop pause --reason "User interrupted"
```

Autopilot may start a loop when the user explicitly requests continuation. It should clarify first, then plan, execute slices, record reviews and triage when useful, write proofs, and close out when Codex judges the PRD is satisfied.

Autopilot should only open a PR when PR intent is part of the PRD.
