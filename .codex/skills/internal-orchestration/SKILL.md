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

## Advisor / CritiqueScout Lanes

Advisor/CritiqueScout lanes are advice-only support lanes for the parent
orchestrator. They are not user-facing slash commands, not formal review
lanes, and not mission-truth writers.

Use an advisor lane when a public workflow needs strategic critique, for
example before a level-5 plan seal, before mission close, after repeated
blocking finding patterns, when the parent is changing approach, or when the
proof/review strategy feels proxy-based.

The parent keeps mission truth, final synthesis, and all durable writeback
decisions.

An advisor brief must include:

- `checkpoint`: stable checkpoint id such as `high_risk_plan_seal`,
  `mission_close_preflight`, `repeated_finding_pattern`, `approach_change`,
  `post_orientation_pre_write`, `proof_strategy_check`, or
  `review_design_check`
- `mission_id` and `target_ref`: explicit boundary such as mission, spec,
  package, bundle, or close boundary
- `question`: the exact strategic question for the advisor
- `bounded_context`: mission artifacts, current route/spec refs, and concise
  parent summary; do not dump unrelated transcript
- `allowed_reads`: the explicit read-only surfaces the advisor may inspect
- `must_not`: no skills, no file mutation, no Ralph lease, no mission-truth
  writes, no review outcome writeback, no gate or closeout writeback, no
  completion judgment, and no user-facing `/advisor` behavior
- `required_checkpoint_handling`: if a required checkpoint is not invoked, the
  parent records one bounded skip reason (`not_applicable`,
  `covered_by_recent_advisor`, `blocked_by_missing_context`, or
  `risk_downgraded_with_evidence`)
- `output_shape`: bounded `advisor-output` JSON or visible advisory note

Advisor output shape:

```json
{
  "artifact": "advisor-output",
  "mission_id": "codex1-product-flow-brainstorm",
  "advisor_lane_id": "advisor_proof_strategy_check_1",
  "checkpoint": "proof_strategy_check",
  "target_ref": "spec:advisor_critiquescout_v1",
  "advice_kind": "critique",
  "context_refs": [
    "PLANS/<mission>/OUTCOME-LOCK.md",
    "PLANS/<mission>/PROGRAM-BLUEPRINT.md"
  ],
  "question": "Does the proof strategy rely on proxy evidence?",
  "summary": "Short advice summary.",
  "risks": [
    {
      "risk": "Proof rows may pass while route truth is stale.",
      "severity": "medium",
      "evidence_refs": ["path/or/artifact"]
    }
  ],
  "recommended_actions": [
    {
      "action": "Repair proof row wording.",
      "rationale": "Why this action helps."
    }
  ],
  "requires_review": false,
  "requires_replan": false,
  "confidence": "high",
  "recorded_at": "RFC3339 timestamp"
}
```

Allowed `advice_kind` values are `critique`, `option_pressure`,
`proof_strategy`, `risk_check`, and `mission_close_preflight`. Allowed
`confidence` values are `low`, `medium`, and `high`.

If the advice materially changes the route, proof strategy, review design, or
close decision, the parent records one disposition in the relevant visible
artifact or receipt:

- `followed`
- `partially_followed`
- `rejected_with_evidence`
- `converted_to_review`
- `converted_to_replan`
- `needs_user`

For non-material advice, the parent may record `no_material_change` in notes or
receipts instead of creating a full disposition artifact.

Advisor output may recommend review, repair, replan, `needs_user`, or a safer
route, but it cannot enact any of those branches. Advisor output is critique,
not formal review evidence. If advisor advice finds a correctness or intent
concern that should block a target, the parent routes that concern into
`$review-loop` or `internal-replan`; the advisor does not clear, fail, or pass
gates directly. If review is owed, the parent must still run `$review-loop`
with bounded findings-only reviewer lanes and durable reviewer-output artifacts.

## Return Shape

Child outputs should be compact, evidence-backed, and easy for the parent to
fold back into visible artifacts or decisions.
