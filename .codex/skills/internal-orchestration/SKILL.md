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

Child agents never acquire Ralph loop leases. Ralph continuation is parent
orchestrator state only; all subagents are Ralph-exempt and may stop normally.
If a child returns no output, partial output, invalid output, or an error, the
parent records that as missing/contaminated/failed evidence and decides whether
to retry, repair, or replan.

For review specifically, parent authority is orchestration authority, not review
judgment authority. The parent may prepare evidence, brief reviewers, aggregate
reviewer outputs, detect contaminated waves, route repair/replan, and perform
durable writeback, but the actual code/spec/intent/integration/mission-close
review judgment must come from reviewer-agent outputs.

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
- use `gpt-5.3-codex` for code bug/correctness reviewer lanes
- use `gpt-5.4` for spec/intent, integration, PRD, and mission-close reviewer
  lanes
- do not use `gpt-5.4-mini` as a default blocking-review model; reserve it only
  for narrow non-blocking support roles when a later plan explicitly allows it
- reserve heavier flagship synthesis for the parent thread unless a child lane
  truly needs it

## Findings-Only Review Children

When `$review-loop` delegates to child reviewers, those children are
findings-only lanes. Their brief must require `NONE` or structured findings and
must forbid skills, mission-truth writeback, gate clearing, and completion
judgment.

The parent must capture review truth before launching findings-only reviewers,
capture a frozen review evidence snapshot for the child brief, and keep the
full review truth snapshot parent-held for writeback. Child reviewers receive
the frozen evidence snapshot only; they do not receive the parent writeback
snapshot. If a child lane mutates mission truth, the parent treats the review
wave as contaminated instead of accepting the child output or clearing the gate.
Child briefs should prefer the frozen evidence snapshot over live mutable repo
paths.

Parent review writeback must cite reviewer-agent output evidence from the
bounded inbox, for example `reviewer-output:<bundle-id>:<output-id>` returned
by `codex1 internal record-reviewer-output`. Do not record a clean or blocking
review disposition from parent-only judgment; if reviewer inbox artifacts are
missing or the wave is contaminated, route that state explicitly instead of
clearing the review boundary.

Reviewer-output evidence is not writeback authority. Child reviewer lanes may
persist only their bounded `NONE` or findings payload through
`record-reviewer-output` and must not call `record-review-outcome`, compile
packages, clear gates, or append closeouts. Runtime writeback rejects
reviewer-lane-like identities and rejects arbitrary reviewer-output strings, so
a child cannot self-clear a gate by turning its result into a mission-truth
mutation.

The parent-held review truth snapshot includes a transient writeback authority
token returned by `capture-review-truth-snapshot`. Persisted review truth files
store only a verifier and are not sufficient for writeback. Do not pass the
plaintext token to child lanes, prompts, notes, evidence snapshots, or other
child-readable artifacts. The parent must capture that authority once per
review bundle before child reviewers run; recapture/remint attempts for the
same bundle are invalid and a fresh review wave requires a fresh bundle.

The active parent loop lease also returns a transient parent authority token.
Parent-only mission mutations during an active authoritative loop must provide
that token through `CODEX1_PARENT_LOOP_AUTHORITY_TOKEN`. Child lanes must not
receive the token; their allowed mutation remains only `record-reviewer-output`.

For blocking review lanes, prefer read-only/explorer-style child agents with
`fork_turns` set to `none`. Do not use mutation-capable worker/default lanes as
reviewers unless a later package proves a stronger isolation mechanism.

## Return Shape

Child outputs should be compact, evidence-backed, and easy for the parent to
fold back into visible artifacts or decisions.
