---
artifact: program-blueprint
mission_id: review-loop-delegated-review-only
version: 1
lock_revision: 1
blueprint_revision: 16
plan_level: 5
risk_floor: 4
problem_size: M
status: approved
proof_matrix:
- claim_ref: claim:delegated-review-authority
  statement: Substantive review judgment is delegated to reviewer agent roles; the parent cannot clear or fail review from self-review.
  required_evidence:
  - delegated authority runtime tests
  - fresh post-isolation review
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - lock:1
  - blueprint
- claim_ref: claim:qualification-regression-guard
  statement: Qualification reruns after the reviewer-lane capability split and guards delegated-review-only behavior.
  required_evidence:
  - qualification test output
  - docs/qualification updates
  review_lenses:
  - release_gate_integrity
  - evidence_adequacy
  governing_contract_refs:
  - blueprint
- claim_ref: claim:review-writeback-authority-token
  statement: Persisted review truth snapshots cannot clear gates without an ephemeral parent-held writeback token that reviewer lanes do not receive.
  required_evidence:
  - runtime token rejection tests
  - snapshot redaction tests
  - review-loop docs
  review_lenses:
  - correctness
  - evidence_adequacy
  - operability_rollback_observability
  governing_contract_refs:
  - contradiction:7de544a7
  - blueprint
- claim_ref: claim:reviewer-lane-canonical-write-isolation
  statement: Child-visible review evidence does not include the parent review truth snapshot writeback capability, and reviewer lanes cannot be driven by parent Ralph continuation.
  required_evidence:
  - runtime evidence snapshot tests
  - review-loop spawn protocol source assertions
  - stop-hook reviewer lane isolation tests
  review_lenses:
  - correctness
  - evidence_adequacy
  - operability_rollback_observability
  governing_contract_refs:
  - contradiction:569eac4f
  - blueprint
- claim_ref: claim:reviewer-output-inbox-contract
  statement: Reviewer lanes can persist bounded NONE/findings outputs to a non-gate inbox, and parent writeback consumes those outputs without allowing child lanes to mutate gates or closeouts.
  required_evidence:
  - runtime inbox tests
  - record-review-outcome evidence tests
  - review-loop docs
  review_lenses:
  - correctness
  - evidence_adequacy
  - operability_rollback_observability
  governing_contract_refs:
  - contradiction:ed548fc6
  - blueprint
- claim_ref: claim:reviewer-parent-writeback-guard
  statement: Reviewer-lane-like identities and self-clear evidence patterns cannot use parent review writeback to clear gates or advance mission truth.
  required_evidence:
  - runtime rejection tests
  - review-loop docs
  - fresh post-guard review
  review_lenses:
  - correctness
  - evidence_adequacy
  - operability_rollback_observability
  governing_contract_refs:
  - contradiction:b91d1f83
  - blueprint
decision_obligations:
- obligation_id: obligation:child-visible-writeback-capability
  question: Should child-visible review evidence snapshots include the full review truth snapshot?
  why_it_matters: The full snapshot lets a reviewer lane attempt parent-owned writeback.
  affects:
  - architecture_boundary
  - proof_design
  - review_contract
  governing_contract_refs:
  - contradiction:569eac4f
  review_contract_refs:
  - review:reviewer_lane_canonical_write_isolation
  mission_close_claim_refs:
  - claim:reviewer-lane-canonical-write-isolation
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - contradiction
  - runtime snapshot schema
  status: selected
  resolution_rationale: Do not export the full review truth snapshot to child evidence; keep it parent-held for writeback.
  evidence_refs:
  - .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:569eac4f-7de8-49f3-b5ef-84ea84eec741
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:reviewer-parent-writeback-boundary
  question: Must review writeback reject reviewer-lane-like identities even when they cite reviewer-output evidence?
  why_it_matters: The observed reviewer lane self-cleared a gate by recording a clean review with a reviewer-output ref while the parent was waiting.
  affects:
  - architecture_boundary
  - proof_design
  - review_contract
  governing_contract_refs:
  - contradiction:b91d1f83
  review_contract_refs:
  - review:reviewer_parent_writeback_guard
  mission_close_claim_refs:
  - claim:reviewer-parent-writeback-guard
  blockingness: critical
  candidate_route_count: 3
  required_evidence:
  - contradiction
  - runtime rejection test
  - docs update
  status: selected
  resolution_rationale: Yes. Reviewer output evidence proves review judgment only; it does not authorize the child lane to call parent writeback. Runtime must reject reviewer-lane-like reviewer identities and self-clear patterns.
  evidence_refs:
  - .ralph/missions/review-loop-delegated-review-only/contradictions.ndjson:b91d1f83-8db2-4c91-9bc9-711ab37a1f70
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:reviewer-spawn-role
  question: Which native reviewer lane role is safe enough for blocking review after contamination?
  why_it_matters: Default/worker reviewer lanes were able to mutate canonical mission truth and appear to be driven by Ralph continuation.
  affects:
  - architecture_boundary
  - review_contract
  - execution_sequencing
  governing_contract_refs:
  - contradiction:569eac4f
  review_contract_refs:
  - review:reviewer_lane_canonical_write_isolation
  mission_close_claim_refs:
  - claim:reviewer-lane-canonical-write-isolation
  blockingness: critical
  candidate_route_count: 3
  required_evidence:
  - review-loop docs
  - stop-hook lane metadata behavior
  status: selected
  resolution_rationale: Use explorer-style findings-only lanes with fork_turns none and frozen evidence only; add stop-hook isolation proof so reviewer lanes return bounded output instead of following parent Ralph prompts.
  evidence_refs:
  - PLANS/review-loop-delegated-review-only/REPLAN-LOG.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
selected_target_ref: spec:reviewer_output_inbox_contract
---
# Program Blueprint

## 1. Locked Mission Reference

- Mission id: `review-loop-delegated-review-only`
- Lock revision: `1`
- Outcome summary: `$review-loop` must be a parent/orchestrator workflow only. Substantive review judgment belongs to reviewer agent roles; the parent may explore, brief, spawn, aggregate, detect contamination, route repair or replan, and write durable outcomes, but must not itself perform code/spec/intent/integration/mission-close review judgment.

## 2. Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | The Outcome Lock forbids parent self-review and assigns review judgment to reviewer agent roles. | `OUTCOME-LOCK.md` | user_lock | ratified lock | lock:1 | current | high |
| T2 | verified_fact | delegated_review_authority_contract produced useful implementation and proof, but later review truth was contaminated by child-lane mutation. | `REPLAN-LOG.md`, `contradictions.ndjson` | repo | artifact read | contradiction 569eac4f | current | high |
| T3 | verified_fact | Current review evidence snapshots export the full review truth snapshot to child-visible evidence, giving reviewer lanes the same writeback capability the parent uses. | `.ralph/.../review-evidence-snapshots/*.json`, `crates/codex1-core/src/runtime.rs` | repo | artifact and source read | blueprint:3 | current | high |
| T4 | constraint | Child reviewers still cannot mutate mission truth or clear gates; parent-owned writeback remains the only durable write path. | `OUTCOME-LOCK.md`, prior capability-boundary mission | prior contract | artifact read | current | current | high |
| T5 | verified_fact | A reviewer-lane retry after the evidence-snapshot split still mutated mission truth by recording a clean review and advancing package truth while the parent was waiting. | `contradictions.ndjson:b91d1f83`, `closeouts.ndjson:50-52`, `REVIEW-LEDGER.md` | repo | artifact read | blueprint:12 | current | high |

## 3. System Model

- Touched surfaces: `$review-loop`, `internal-orchestration`, runtime backend docs, Multi-Agent V2 guide, review evidence snapshot schema, review truth snapshot flow, record-review-outcome preconditions, runtime tests, and qualification handoff.
- Boundary summary: reviewer agents may receive bounded evidence and return `NONE` or findings. The parent alone retains the review truth snapshot capability needed for durable writeback.
- Hidden coupling summary: a child-visible evidence snapshot that contains `review_truth_snapshot` lets a reviewer lane accidentally call the same writeback command as the parent. The route must split child evidence from parent writeback capability before review can be trusted.

## 4. Invariants And Protected Behaviors

- No parent-local review judgment may clear or fail a review boundary.
- No child reviewer may receive the parent writeback capability needed to call record-review-outcome successfully.
- Review evidence snapshots are child-readable evidence, not authority tokens.
- Review truth snapshots are parent-held writeback guards and must not be embedded in child reviewer briefs.
- Reviewer lanes must be spawned as findings-only explorer/read-only lanes with fork_turns none and frozen evidence snapshots only.
- If reviewer outputs are missing or mission truth changes during review, the wave routes to contamination/replan rather than clean review.
- Review writeback must reject reviewer-lane identities and reviewer-lane self-clear patterns; only parent/orchestrator writeback may update gates, closeouts, ledgers, specs, or mission-close artifacts.

## 5. Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | Child-visible review evidence snapshots omit the full parent review truth snapshot while preserving enough non-capability guard metadata for validation. | runtime tests | reviewer_lane_canonical_write_isolation | yes |
| P2 | record-review-outcome still requires a parent-supplied review truth snapshot and reviewer-output evidence before clearing review. | runtime tests | reviewer_lane_canonical_write_isolation | yes |
| P3 | Public workflow docs require explorer-style findings-only reviewer lanes, fork_turns none, and no mutation-capable default/worker reviewer lanes for blocking review. | source assertions | reviewer_lane_canonical_write_isolation | yes |
| P4 | Preserved delegated_review_authority_contract implementation remains green under the new split. | regression tests | reviewer_lane_canonical_write_isolation | yes |
| P5 | Qualification is rerun only after the reviewer lane capability split lands. | qualification receipt | delegated_review_qualification_guard | yes |
| P6 | Reviewer-lane-like identities cannot call parent review writeback or self-clear gates with reviewer-output refs. | runtime tests plus docs | reviewer_parent_writeback_guard | yes |
| P7 | Persisted review truth snapshots are not sufficient writeback authority; parent writeback requires an ephemeral token not stored in child-readable artifacts. | runtime tests plus docs | review_writeback_authority_token | yes |

## 6. Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Should the child evidence snapshot include the full review truth snapshot? | Including it lets child reviewer lanes use the parent writeback capability. | critical | resolved: no, child evidence gets non-capability guard metadata only | contradiction 569eac4f |
| DO-2 | Should we retry reviewer lanes with stronger prompts only? | Prompt-only failed and allowed mission truth mutation. | critical | rejected: add capability split and explorer-only spawn protocol first | REPLAN-LOG.md |
| DO-3 | Can valid implementation work be preserved? | The code/proof from delegated_review_authority_contract still appears useful, but its review/qualification advancement is contaminated. | major | resolved: preserve implementation receipts, invalidate contaminated review/qualification closeouts | REPLAN-LOG.md |
| DO-4 | Is prompt-only or explorer-only reviewer isolation sufficient? | A reviewer lane still wrote clean review/package truth after bounded waits and interrupt. | critical | rejected: add runtime parent-only writeback guard before rerunning qualification review | contradiction b91d1f83 |

## 7. In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| delegated_review_authority_contract | preserved_implementation_pending_revalidation | The implementation evidence is useful, but its later review advancement was contaminated. | revalidated after isolation guard | yes |
| reviewer_lane_canonical_write_isolation | runnable_frontier | Split child reviewer evidence from parent writeback capability and require safer reviewer spawn protocol. | code correctness plus spec/intent review | yes |
| delegated_review_qualification_guard | preserved_package_pending_revalidation | Qualification proof/package are useful, but clean review 71f23706 is contaminated by child writeback and must be rerun after the parent writeback guard. | qualification review after guard | yes |
| reviewer_parent_writeback_guard | preserved_package_pending_revalidation | Identity guard implementation is useful, but its clean review was contaminated by parent impersonation and must be revalidated after token authority lands. | runtime correctness and evidence review after token guard | yes |
| review_writeback_authority_token | preserved_package_pending_revalidation | Token implementation is useful but token minting remains child-accessible through the CLI; revalidate after reviewer-output inbox lands. | runtime correctness and evidence review after inbox | yes |
| reviewer_output_inbox_contract | runnable_frontier | Add a bounded child-writeable reviewer output inbox and require parent review writeback to consume inbox outputs instead of child chat/self-writeback. | runtime correctness and evidence review | yes |
| contaminated_review_advancement | invalidated | closeouts 7-22 advanced mission truth from child-lane mutation and cannot prove clean state. | replan log | no |

## 8. Option Set

- Option A: retry the same reviewer agent prompts and hope they comply.
- Option B: use explorer-only reviewer prompts but leave child-visible writeback snapshots in evidence.
- Option C: split child review evidence from parent writeback capability, require safer reviewer spawn protocol, then revalidate.
- Option D: keep evidence-snapshot split and add a runtime parent-only review writeback guard that rejects reviewer-lane identities/self-clear patterns before rerunning qualification review.

## 9. Selected Architecture

Select Option D after contradiction `b91d1f83`. Option C remains preserved as a necessary lower layer, but the reproduced failure proves it is not sufficient. The new route keeps capability separation and adds a runtime parent-only writeback guard: reviewer lanes may return bounded outputs, but calls that look like reviewer-lane self-writeback must be rejected by `record-review-outcome` before they can mark gates clean or advance downstream packages.

Previously selected Option C. The key route change is capability separation. Child reviewers get a frozen evidence snapshot that contains bundle bindings, proof rows, receipts, changed-file context, reviewer instructions, and non-capability guard metadata. They do not receive the full `ReviewTruthSnapshot` needed by `record-review-outcome`. The parent captures and retains that snapshot separately, aggregates reviewer outputs, verifies mission truth has not drifted, and only then records outcome with reviewer-output evidence. Public workflow docs must also require explorer/read-only reviewer lanes with `fork_turns` set to none for blocking reviews, so reviewer children are not launched as mutation-capable worker/default lanes.

## 10. Rejected Alternatives And Rationale

- Prompt-only retry: rejected because reviewer lanes already mutated mission truth under prompt-only restrictions.
- Exporting full review truth snapshot to child evidence: rejected because it turns the child brief into a writeback capability.
- Accepting contaminated closeouts as clean because later gates passed: rejected because it hides the exact failure the mission is correcting.
- Platform-only sandboxing: deferred because the repo cannot assume native platform permissions that are not available, but the route still narrows accidental child writeback.
- Explorer/default retry after missing reviewer outputs: rejected because retry reproduced mission-truth mutation and contaminated closeouts 50-52.

## 11. Migration / Rollout / Rollback Posture

- Migration posture: schema hardening around review evidence snapshots, runtime parent-only writeback identity checks, and workflow doc tightening. Existing parent-held truth snapshot writeback stays intact for parent/orchestrator callers only.
- Rollout posture: land capability split, rerun delegated authority regressions, then rerun qualification and review from fresh bundles.
- Rollback posture: if hidden platform support later provides true read-only reviewer lanes, preserve the parent writeback capability split and simplify prompt restrictions only with proof.

## 12. Review Bundle Design

- Mandatory review lenses: correctness, spec_conformance, evidence_adequacy, interface_compatibility, operability_rollback_observability.
- Required receipts: runtime tests for evidence snapshot redaction, record-review-outcome parent snapshot requirement, reviewer spawn protocol source assertions, delegated authority regression tests, and artifact validation.
- Required changed-file context: `.codex/skills/review-loop/SKILL.md`, `.codex/skills/internal-orchestration/SKILL.md`, `docs/runtime-backend.md`, `docs/MULTI-AGENT-V2-GUIDE.md`, `crates/codex1-core/src/runtime.rs`, `crates/codex1-core/src/lib.rs`, `crates/codex1/src/internal/mod.rs`, `crates/codex1/tests/runtime_internal.rs`.
- Mission-close claims requiring integrated judgment: review judgment delegated to reviewer agents, reviewer agents lack parent writeback capability, contaminated child-lane advancement is invalidated, and qualification reruns after the capability split.

## 13. Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| delegated_review_authority_contract | Preserve implemented delegated-review outcome authority contract for revalidation. | complete | solo | none |
| reviewer_lane_canonical_write_isolation | Prevent findings-only reviewer lanes from mutating canonical mission truth during review-loop orchestration. | runnable | solo | none |
| reviewer_parent_writeback_guard | Prevent reviewer-lane self-writeback after bounded review output failures. | runnable | solo | reviewer_lane_canonical_write_isolation |
| delegated_review_qualification_guard | Rerun qualification after the reviewer-lane capability split and parent writeback guard. | runnable | solo | reviewer_lane_canonical_write_isolation, reviewer_parent_writeback_guard |

## 14. Execution Graph And Safe-Wave Rules

- Graph summary: preserve completed `reviewer_lane_canonical_write_isolation`, execute `reviewer_output_inbox_contract`, then revalidate `review_writeback_authority_token` and `reviewer_parent_writeback_guard`, rerun qualification/review from fresh bundles only, then mission-close review.
- Safe-wave rule 1: do not run reviewer child lanes for blocking review with default/worker agent types while this isolation guard is unfinished.
- Safe-wave rule 2: do not accept closeouts 7-22 as proof of clean review; they are preserved only as contamination evidence.
- Safe-wave rule 3: do not pass full review truth snapshots to child reviewer prompts or child-visible evidence snapshots.
- Safe-wave rule 4: do not accept gate `71f23706`, closeout `50`, or later mission-level package attempts `51-52` as clean proof; they are contamination evidence for contradiction `b91d1f83`.
- Safe-wave rule 5: do not accept gate `55acf452` or closeout `57` as clean proof; they show that repo-visible review truth snapshots can be used for parent impersonation without an ephemeral token.
- Safe-wave rule 6: do not accept gate `de12d0dd` or closeout `64` as clean proof; reviewer lanes can still mint tokens through child-accessible CLI commands, so review outputs need a bounded inbox before parent writeback.

## 15. Risks And Unknowns

- This is not a malicious-security sandbox against a child with full filesystem access; it is a product capability split to prevent accidental child writeback under native Codex review orchestration.
- Existing validation code may assume `review_truth_snapshot` is embedded in evidence snapshots and will need compatible schema migration.
- Reviewer spawn protocol still relies partly on public workflow discipline unless the native platform exposes hard read-only reviewer roles.
- Runtime identity checks are not a malicious security sandbox, but they prevent the observed accidental reviewer-lane self-writeback path and force contaminated waves back to replan.

## 16. Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-4 | Reopen blueprint after reviewer lane mutation contamination. | The route changed: reviewer evidence and parent writeback capability must split. | contradiction 569eac4f | blueprint, specs, graph | 4 |
| D-5 | Child-visible evidence snapshots must not embed the full review truth snapshot. | It can be used as parent writeback capability. | gates/closeouts contamination | runtime, tests, docs | 4 |
| D-6 | Blocking reviewer lanes use explorer-style findings-only spawn protocol with fork_turns none. | Default mutation-capable agents were unsafe for review lanes. | live failure | review-loop docs, orchestration docs | 4 |
| D-7 | Runtime writeback must reject reviewer-lane self-writeback identities before a gate can be cleared. | Evidence snapshot split did not stop reviewer lane from recording clean review and advancing truth. | contradiction b91d1f83 | runtime, tests, docs | 13 |
| D-8 | Parent writeback requires an ephemeral authority token whose plaintext is not persisted in child-readable artifacts. | Reviewer lanes can impersonate parent identity and read persisted review truth snapshots from repo files. | contradiction 7de544a7 | runtime, CLI, tests, docs | 15 |

## 17. Replan Policy

- Reopen Outcome Lock when: the user decides child reviewer writeback is acceptable or native Codex cannot support reviewer-agent delegation at all.
- Reopen blueprint when: reviewer capability split proves insufficient, parent-only writeback guard proves insufficient, or platform read-only lanes change the route.
- Reopen execution package when: write scope, schema details, or proof obligations change while preserving capability split architecture.
- Local repair allowed when: tests expose missing redaction, doc wording gaps, or validation compatibility issues within the selected route.
