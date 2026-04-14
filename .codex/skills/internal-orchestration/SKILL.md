---
name: internal-orchestration
description: Internal Codex1 native-subagent orchestration support. Use when a public workflow needs bounded multi-agent help for scouting, critique, drafting, review, or other parallel support work without giving up parent ownership of mission truth.
---

# Internal Orchestration

This is the bounded native-subagent method for Codex1.

Ground this skill in `docs/MULTI-AGENT-V2-GUIDE.md` whenever you need deeper
implementation detail.

## Parent Authority

The parent thread owns:

- mission truth
- final synthesis
- completion judgment
- artifact writeback and reconciliation

Child agents do bounded work only.

## Safe Defaults

- prefer read-heavy delegation first
- default to conservative `fork_turns`; do not dump full transcript history into
  every lane
- keep V1 depth at `1`
- close agents when their result is integrated or no longer needed
- reconcile from artifacts plus live-agent inspection on resume

## Spawn Brief Contract

Every delegated task should name:

- the exact question or artifact to produce
- the allowed repo surface
- the expected output shape
- what the child must not decide
- the evidence refs it must return
- the file ownership boundary if it may edit

If you cannot write a crisp brief, you are not ready to delegate that subtask.

## Naming and Tool Use

- use stable lowercase `task_name` values with digits or underscores only
- treat queue-only messaging and turn-triggering follow-up as different actions
- in the current development wrapper, `followup_task` is the surfaced alias for
  the PRD's conceptual `assign_task`
- treat `wait_agent` as mailbox-edge waiting only, not proof of completion or
  payload truth

## Parallelism Rules

- sibling read-only scouts may fan out broadly when their tasks are independent
- same-workspace writes serialize unless graph safety and path disjointness are
  explicit
- unknown overlap means serialize, not parallelize
- one child must never silently revert another child's work

## Model Routing Guidance

- use `gpt-5.3-codex-spark` or another fast lane for broad, text-only scouting
- use `gpt-5.4-mini` for bounded correctness-heavy helpers such as QA or review
  prep
- reserve heavier flagship synthesis for the parent thread unless a child lane
  truly needs it

## Return Shape

Child outputs should be compact, evidence-backed, and easy for the parent to
fold back into visible artifacts or decisions.
