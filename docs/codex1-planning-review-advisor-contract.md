# Codex1 Planning, Review, and Advisor Contract

**Status:** product contract.
**Purpose:** define the auditable registries and decision rules behind `$plan`, `$execute`, `$review-loop`, `$autopilot`, and Advisor/CritiqueScout.
**Source of truth:** complements `docs/codex1-prd.md` and `docs/codex1-ux-flow.md`.

## 1. Contract Shape

Codex1 should keep the user experience natural while making internal decisions auditable.

The user may say:

```text
$plan make the most thorough plan in your life
```

or:

```text
$plan keep this compact
```

Codex1 should not require users to memorize flags for normal use. Internally, the system must still record why it chose a given planning depth, review weight, advisor checkpoint, and repair/replan branch.

This doc defines four registries:

- `PlanningRigorRegistry`
- `ReviewProfileRegistry`
- `AdvisorCheckpointRegistry`
- `RepairReplanDecisionRegistry`

These are product contracts. They may later become code/data registries, but the behavior is normative here.

## 2. PlanningRigorRegistry

Planning rigor is computed as:

```text
effective_rigor = max(user_requested_rigor, mission_risk_floor)
```

### Rigor Levels

| Level | Name | Meaning |
| --- | --- | --- |
| 1 | compact | bounded local change, minimal coupling, direct proof |
| 2 | standard | subsystem change, explicit proof/review, some coupling |
| 3 | thorough | serious system change, full proof matrix, critiques, execution graph when needed |
| 4 | deep | migration/interface/ops-coupled work, rollout/rollback/contingency |
| 5 | max | mission-critical, core runtime, authority/security, global setup, or rollback-limited work |

Natural language maps to a requested floor:

| User wording | Requested floor |
| --- | --- |
| "quick", "tiny", "compact" | 1 |
| normal `$plan` with no modifier | risk floor |
| "thorough", "really think this through" | 3 |
| "deep", "state of the art", "spend compute" | 4 |
| "most thorough plan in your life", "no token limit", "perfect plan" | 5 |

### Risk Triggers

Risk triggers raise the floor regardless of user wording.

| Trigger | Minimum floor |
| --- | --- |
| one-file low-risk change with obvious proof | 1 |
| multi-file code change | 2 |
| cross-module behavior change | 3 |
| public API or interface contract change | 3 |
| review authority, Ralph, closeout, or mission truth change | 5 |
| global setup, hooks, restore, uninstall, backup, or user config | 5 |
| security, auth, billing, data loss, migration, or destructive action | 5 |
| rollback-limited or hard-to-test behavior | 4 |
| repeated review findings in same class | 4 |
| unclear user intent or protected surfaces | return to `$clarify` or run bounded clarify/advisor before planning |

### Planning Rigor Record

Every non-trivial `$plan` cycle should leave an auditable planning rigor record in the blueprint or support artifact.

Minimum shape:

```yaml
planning_rigor:
  user_requested_rigor: compact | standard | thorough | deep | max
  mission_risk_floor: compact | standard | thorough | deep | max
  effective_rigor: compact | standard | thorough | deep | max
  risk_triggers:
    - global_setup
    - hook_authority
  required_methods:
    - truth_register
    - system_map
    - invariant_register
    - proof_matrix
    - review_design
  methods_run:
    - truth_register
    - system_map
  methods_skipped:
    - method: alternative_generation
      reason: only_one_viable_route_after_repo_constraints
  subagents_used:
    - role: repo_surface_scout
      purpose: map setup/restore surfaces
  advisors_used:
    - checkpoint: high_risk_plan_seal
      disposition: followed | rejected_with_evidence | escalated_to_replan
```

Planning is not complete just because a blueprint file exists. Planning is complete only when the effective rigor's required methods have been run, skipped with honest rationale, or converted into explicit proof-gated spikes.

## 3. Required Planning Methods

| Method | Required when | Purpose |
| --- | --- | --- |
| truth register | level 2+ | separate user facts, repo facts, and Codex inferences |
| system map | level 2+ | identify surfaces and dependencies |
| boundary/coupling map | level 3+ or multi-surface work | prevent hidden coupling surprises |
| invariant register | level 3+ or contract-rich work | name what must remain true |
| proof matrix | level 2+ | define how success is proven |
| review design | level 2+ | define required review boundaries and lanes |
| option generation | only when real decision obligations exist | compare viable routes without fake alternatives |
| adversarial critique | level 3+ | attack route before execution inherits it |
| advisor checkpoint | level 4+, level 5, repeated findings, or mission close | get strategic second opinion |
| execution graph | non-trivial sequencing or parallelism | make dependencies explicit |
| package next target | always before `$execute` | bind execution to fresh package truth |

## 4. What Makes A Plan "Perfect Enough"

Codex1 should not claim a perfect plan in an absolute sense. It should claim an execution-ready plan when:

- user intent is locked enough
- protected surfaces and non-goals are explicit
- critical decisions are resolved or proof-gated
- selected route has survived critique proportionate to risk
- specs are bounded and executable
- proof rows are concrete
- review requirements are explicit
- replan boundaries are explicit
- the next target has a fresh passed execution package
- execution does not need to invent architecture, proof, or review structure

For level-5 planning, "perfect enough" additionally requires:

- explicit rejected alternatives or why no alternative survived
- advisor/CritiqueScout before final plan seal
- explicit rollback/restore/backup posture when relevant
- mission-close proof and integrated review claims identified early

## 5. ReviewProfileRegistry

Review is selected from the boundary, touched surfaces, and risk.

| Profile | When required | Default model | Required output |
| --- | --- | --- | --- |
| `code_bug_correctness` | code-producing slice at proof-worthy checkpoint | `gpt-5.3-codex` | P0/P1/P2/P3 findings or `NONE` |
| `local_spec_intent` | one spec/phase reaches completion boundary | `gpt-5.4` | intent/spec conformance findings or `NONE` |
| `integration_intent` | multiple related slices/phases combine | `gpt-5.4` | cross-slice coherence findings or `NONE` |
| `mission_close` | before terminal completion | `gpt-5.4` | mission-level findings or `NONE` |
| `targeted_repair` | after a repair for specific finding class | profile matching the repaired risk | findings on repaired scope only |

Default lane counts:

| Boundary | Minimum lanes |
| --- | --- |
| small non-code doc/proof update | 1 spec/intent lane when review owed |
| code-producing low/medium-risk slice | 1 code lane, plus spec/intent lane when spec satisfaction is non-trivial |
| substantial code-producing slice | 2 code lanes + 1 spec/intent lane |
| integration boundary | 1-2 integration/intent lanes |
| mission close | 2 mission-close lanes minimum |
| core Ralph/review/authority/global setup change | at least code + spec/intent + advisor checkpoint |

Clean review requires:

- every required lane persisted bounded reviewer-output
- no P0/P1/P2 findings remain
- review evidence snapshot is fresh
- review wave was not contaminated
- parent records the outcome, not reviewer lanes

## 6. Reviewer Prompt Contract

Reviewer prompts should be generated from the review profile rather than improvised from scratch.

Minimum reviewer prompt fields:

- profile
- model expectation
- target bundle/scope
- required evidence snapshot
- severity scale
- output schema
- forbidden actions
- exact persistence instruction for reviewer-output

Reviewer forbidden actions:

- do not invoke skills
- do not mutate files
- do not record review outcomes
- do not clear gates
- do not decide mission completion
- do not acquire Ralph loop leases
- do not withhold findings because parent mission state is blocked

Reviewer output schema:

```text
NONE
```

or:

```json
{
  "findings": [
    {
      "severity": "P1",
      "finding_class": "interface_contract",
      "title": "Short title",
      "evidence_refs": ["path:line"],
      "rationale": "Why this blocks cleanliness.",
      "suggested_next_action": "repair | replan | rerun_profile"
    }
  ]
}
```

Reviewer-output artifacts should declare:

- `reviewer_lane`
- `review_profile`
- `finding_class` for each finding
- `bundle_id`
- `evidence_snapshot_fingerprint`
- `recorded_at`

## 7. AdvisorCheckpointRegistry

Advisor/CritiqueScout is parent-callable strategic critique.

It is not:

- a user slash command
- a formal review lane
- a replacement for `$review-loop`
- a mission-truth writer

It is:

- callable by the parent orchestrating Codex session
- read-only/advice-only
- Ralph-exempt
- bounded by mission artifacts and relevant transcript context
- durable when its advice affects a decision

### Invocation Contract

The parent may invoke Advisor/CritiqueScout directly, or through a bounded
`internal-orchestration` advisor lane. The invocation must be explicit enough
that another parent can audit why advice was requested.

Minimum invocation fields:

- `checkpoint`: one value from the required or recommended checkpoint tables
- `mission_id`
- `target_ref`: mission, spec, wave, package, bundle, or decision boundary
- `question`: the concrete strategic question for the advisor
- `context_refs`: bounded mission artifacts or transcript summaries the
  advisor may read
- `forbidden_actions`: no file mutation, no lease acquisition, no gate or
  closeout writeback, no review outcome writeback
- `expected_output`: `advisor-output` JSON or a visible advisory note

Advisor lanes must not receive parent loop authority tokens, review writeback
tokens, or mutation-capable writer packets. If advice requires a code edit,
gate change, review outcome, replan, or user decision, the advisor recommends
that route; the parent performs the authorized action in the proper workflow.

### Required Checkpoints

| Checkpoint | Required when | Purpose |
| --- | --- | --- |
| `high_risk_plan_seal` | effective rigor level 5 or core authority/global setup work | catch weak route before blueprint hardens |
| `mission_close_preflight` | before mission-close review / final closeout | catch premature closeout or missing integrated claims |
| `repeated_finding_pattern` | same class of blocking findings repeats | decide repair vs replan earlier |
| `approach_change` | parent is about to change architecture/strategy | avoid silent drift |

### Recommended Checkpoints

| Checkpoint | When useful |
| --- | --- |
| `post_orientation_pre_write` | before substantial edits after repo reading |
| `proof_strategy_check` | when proof rows may be proxy-based |
| `review_design_check` | when review lanes/profile choice is ambiguous |
| `global_setup_safety_check` | before setup/restore/uninstall/backup changes |

Required checkpoints must either invoke Advisor/CritiqueScout or record a
bounded skip record. Acceptable skip reasons are:

- `not_applicable`: the checkpoint trigger no longer applies after fresh
  mission truth reconciliation
- `covered_by_recent_advisor`: a recent advisor-output covers the same target,
  checkpoint, and unchanged governing refs
- `blocked_by_missing_context`: the parent cannot provide a bounded truthful
  context and must route to planning, replan, or `needs_user`
- `risk_downgraded_with_evidence`: the parent records evidence that the work no
  longer meets the required-checkpoint trigger

Skipping a required checkpoint without one of those records is a contract
violation. Recommended checkpoints may be skipped without a record, but the
parent should still record rationale when the checkpoint is high-risk or likely
to be questioned during review.

### Advisor Output Artifact

Durable advisor advice uses an `advisor-output` artifact or an equivalent
visible advisory note. JSON artifacts should use this shape:

```json
{
  "artifact": "advisor-output",
  "mission_id": "codex1-product-flow-brainstorm",
  "advisor_lane_id": "advisor_high_risk_plan_seal_1",
  "checkpoint": "high_risk_plan_seal",
  "target_ref": "mission:codex1-product-flow-brainstorm",
  "advice_kind": "critique",
  "context_refs": [
    "PLANS/<mission>/OUTCOME-LOCK.md",
    "PLANS/<mission>/PROGRAM-BLUEPRINT.md"
  ],
  "question": "What route or proof risks should the parent address before sealing?",
  "summary": "Short advice summary.",
  "risks": [
    {
      "risk": "A missing proof row may let stale package truth pass.",
      "severity": "medium",
      "evidence_refs": ["PLANS/<mission>/PROGRAM-BLUEPRINT.md"]
    }
  ],
  "recommended_actions": [
    {
      "action": "Add package freshness proof before sealing.",
      "rationale": "Prevents stale execution authorization."
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
`confidence` values are `low`, `medium`, and `high`. Allowed risk severities
are `low`, `medium`, and `high`.

Advisor output is not formal review evidence. It may be cited as planning,
repair, replan, or closeout rationale, but a review gate still requires
findings-only reviewer-output and parent-owned review writeback.

### Advisor Disposition

Parent must record one disposition for material advisor advice:

- `followed`
- `partially_followed`
- `rejected_with_evidence`
- `converted_to_review`
- `converted_to_replan`
- `needs_user`

Advisor advice should not silently override repo evidence or user intent. If advisor and parent evidence conflict, parent should reconcile explicitly rather than switching branches without a record.

Minimum disposition record:

```json
{
  "artifact": "advisor-disposition",
  "mission_id": "codex1-product-flow-brainstorm",
  "advisor_output_ref": "advisor-output:<id-or-path>",
  "checkpoint": "high_risk_plan_seal",
  "disposition": "followed",
  "rationale": "Why the parent accepted, rejected, or routed the advice.",
  "resulting_action": "Updated proof matrix before package compilation.",
  "evidence_refs": ["advisor-output:<id-or-path>", "PLANS/<mission>/PROGRAM-BLUEPRINT.md"],
  "recorded_at": "RFC3339 timestamp"
}
```

Disposition is required when advice materially changes a plan, review route,
repair/replan branch, closeout posture, or user-facing recommendation. If the
advisor returns no material recommendation, the parent may record `no_material_change`
in notes or receipts instead of a full disposition artifact.

### Boundary Tests

Advisor/CritiqueScout V1 is considered implemented only when proof or
qualification evidence demonstrates:

- required checkpoints are invoked or explicitly skipped with an allowed reason
- advisor-output has bounded context, stable schema, and durable evidence refs
- advisor lanes cannot acquire parent leases, write review outcomes, clear
  gates, or mutate canonical mission truth
- parent dispositions exist for material advice
- clean review still depends on `$review-loop` reviewer-output, not advisor
  advice

## 8. RepairReplanDecisionRegistry

The base rule:

```text
repair when the current spec/blueprint/lock remains true
replan when the current contract is wrong or insufficient
```

### Finding Classes

| Finding class | Default branch |
| --- | --- |
| local_code_bug | repair |
| missing_test_or_receipt | repair |
| doc_or_template_drift | repair |
| interface_contract | repair once, replan on repeat or scope mismatch |
| proof_proxy | repair proof if local, replan if proof strategy is wrong |
| review_contract_gap | replan review contract |
| package_scope_wrong | repackage or replan package |
| blueprint_decomposition_wrong | replan blueprint |
| outcome_scope_mismatch | reopen lock / clarify |
| authority_mode_violation | replan authority contract or repair runtime if contract is clear |
| repeated_same_class | replan after threshold |
| malformed_reviewer_output | respawn lane up to cap, then needs_user or replan review contract |

### Loop Caps

Review-loop cap:

- six consecutive non-clean review loops on the same boundary require replan
- targeted repair does not reset the count unless the governing contract changes materially

Earlier replan triggers:

- same finding class repeats three times
- finding says current spec cannot express required behavior
- proof strategy proves a proxy rather than product truth
- repair would exceed package scope
- reviewer/advisor exposes wrong decomposition
- user adds scope that changes the outcome or blueprint

## 9. Deterministic vs Codex Judgment

Deterministic backend should decide:

- current branch from durable mission truth
- artifact/gate freshness
- package validity
- required fields and legal state transitions
- whether review lanes answered
- whether clean review can be accepted
- whether a parent authority token/capability is valid
- whether stop/yield is allowed

Codex judgment should decide:

- architecture and route design
- which repo evidence matters
- how to synthesize critique
- how to brief subagents
- whether advisor advice is compelling
- which local repair best satisfies the spec
- how to communicate tradeoffs to the user

The system should not pretend Codex judgment is deterministic. It should make Codex judgment inspectable, bounded, and reviewable.

## 10. Minimal Next Implementation Contracts

Before large refactors, define and test:

- `PlanningRigorRecord`
- `ReviewProfileRegistry`
- `ReviewerLane` / `finding_class` in reviewer-output
- `can_accept_clean(...)`
- `repair_or_replan(...)`
- `AdvisorCheckpointRegistry`
- scoped parent capability model

These are the bridge between the simple user flow and the reliable machine behavior.
