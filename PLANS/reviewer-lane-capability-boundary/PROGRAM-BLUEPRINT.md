---
artifact: program-blueprint
mission_id: reviewer-lane-capability-boundary
version: 1
lock_revision: 1
blueprint_revision: 6
plan_level: 5
risk_floor: 5
problem_size: M
status: approved
proof_matrix:
- claim_ref: claim:frozen-evidence
  statement: Reviewer evidence snapshots reduce live mutable repo review context while preserving review quality.
  required_evidence:
  - RECEIPTS/reviewer-evidence-snapshot-proof.txt
  review_lenses:
  - evidence_adequacy
  - spec_conformance
  governing_contract_refs:
  - blueprint
- claim_ref: claim:mutation-guard
  statement: Contaminated child reviewer mutation is detected and cannot silently clear review truth.
  required_evidence:
  - RECEIPTS/reviewer-lane-mutation-guard-proof.txt
  review_lenses:
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - lock:1
  - blueprint
- claim_ref: claim:parent-writeback
  statement: Clean child findings-only output still flows through parent-owned review outcome recording.
  required_evidence:
  - RECEIPTS/parent-writeback-proof.txt
  review_lenses:
  - correctness
  governing_contract_refs:
  - lock:1
  - blueprint
- claim_ref: claim:qualification
  statement: Qualification guards reviewer-lane capability boundary against regression.
  required_evidence:
  - RECEIPTS/reviewer-capability-qualification-proof.txt
  review_lenses:
  - release_gate_integrity
  - evidence_adequacy
  governing_contract_refs:
  - blueprint
decision_obligations:
- obligation_id: obligation:evidence-context
  question: Should child reviewers use frozen evidence snapshots or live repo paths by default?
  why_it_matters: It affects review quality and mutation risk.
  affects:
  - proof_design
  - review_contract
  governing_contract_refs:
  - blueprint
  review_contract_refs:
  - review:reviewer_evidence_snapshot_contract
  mission_close_claim_refs:
  - claim:frozen-evidence
  blockingness: major
  candidate_route_count: 2
  required_evidence:
  - review bundle design
  status: selected
  resolution_rationale: Use frozen snapshots by default where practical, with guarded live paths only when needed.
  evidence_refs:
  - PROGRAM-BLUEPRINT.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:guard-route
  question: Should the first fix be mutation detection plus rejection rather than prompt-only enforcement?
  why_it_matters: It determines whether the live failure can recur silently.
  affects:
  - architecture_boundary
  - review_contract
  governing_contract_refs:
  - lock:1
  - blueprint
  review_contract_refs:
  - review:reviewer_lane_mutation_guard
  mission_close_claim_refs:
  - claim:mutation-guard
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - live incident
  - OUTCOME-LOCK.md
  status: selected
  resolution_rationale: Prompt-only enforcement already failed; mutation detection plus parent-owned writeback is the strongest repo-local route.
  evidence_refs:
  - OUTCOME-LOCK.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
selected_target_ref: mission:reviewer-lane-capability-boundary
---
# Program Blueprint

## 1. Locked Mission Reference

- Mission id: `reviewer-lane-capability-boundary`
- Lock revision: `1`
- Outcome summary: Findings-only reviewer lanes must not be able to silently mutate Codex1 mission truth, clear gates, record review outcomes, or terminalize missions. Parent `$review-loop` remains the only review writeback authority.

## 2. Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | Prompt-only reviewer restrictions failed in live use: child review lanes cleared gates and mission-close truth instead of returning findings-only output. | Current conversation and manual-clarify mission artifacts | live incident | observed child lane notifications plus gate/state writeback | current | current | high |
| T2 | verified_fact | Existing `$review-loop` docs already say child reviewers must not mutate truth, but runtime enforcement only bypasses Stop-hook blocking for child result delivery. | `.codex/skills/review-loop/SKILL.md`, `crates/codex1/src/internal/mod.rs` | repo | file read | current dirty tree | current | high |
| T3 | constraint | Native subagent platform permissions may not expose a true read-only sandbox from this repo, so Codex1 must at least detect and reject contaminated review waves durably. | Outcome Lock | inference | lock synthesis from user constraints and platform limits | lock:1 | current | medium-high |

## 3. System Model

- Touched surfaces: `$review-loop`, `internal-orchestration`, Stop-hook lane metadata, review bundles, `record-review-outcome`, gates, closeouts, state, review ledgers, specs, and qualification tests.
- Boundary summary: child reviewers may produce only raw outputs; parent review-loop owns aggregation, mutation validation, review outcome recording, and mission completion judgment.
- Hidden coupling summary: today the child lane can share the same repo/tool authority as the parent, so prompt wording and Stop-hook bypass are insufficient unless parent records a pre-review snapshot and rejects unexpected mission-truth mutations.

## 4. Invariants And Protected Behaviors

- A child reviewer output cannot itself be sufficient evidence to pass a gate; parent aggregation and parent-owned writeback are required.
- Any change to gates, closeouts, state, review ledgers, specs, receipts, bundles, or mission-close artifacts during child review execution must be attributed to parent-owned writeback or treated as contamination.
- Findings-only Stop-hook bypass permits returning a bounded review payload only; it does not grant writeback authority.
- Clean review remains no P0/P1/P2 findings in the latest parent-aggregated loop.

## 5. Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | A simulated child reviewer mutation before parent aggregation is detected and blocks or rejects review clean writeback. | runtime integration test | reviewer_lane_mutation_guard | yes |
| P2 | Parent-owned clean review still records a gate pass when no child mutation contamination exists. | runtime integration test | reviewer_lane_mutation_guard | yes |
| P3 | Stop-hook child-lane result delivery remains non-blocking while parent mission gates still block parent progress. | regression test | reviewer_lane_mutation_guard | yes |
| P4 | Reviewer evidence snapshot/export design removes live mutable repo paths from default child briefs where practical. | docs/tests | reviewer_evidence_snapshot_contract | yes |
| P5 | Qualification includes the contamination case so future review-lane regressions cannot claim green. | qualification receipt | reviewer_capability_qualification | yes |

## 6. Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Prevention-only or detection-plus-rejection? | Repo cannot necessarily control native subagent OS/tool permissions, so pretending to fully prevent writes would be dishonest. | critical | resolved: implement detection-plus-rejection first, with frozen evidence packaging next | Outcome Lock, live incident |
| DO-2 | Should reviewer lanes keep using live repo paths? | Live paths give useful review power but also let children mutate truth. | major | resolved: first guard live mutations, then add frozen evidence snapshot contract to reduce live-path need | this blueprint |
| DO-3 | Who may call `record-review-outcome`? | Determines whether a child can clear gates. | critical | resolved: parent review-loop only; implementation must add machine-checkable authority or contamination guard | Outcome Lock |

## 7. In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| reviewer_lane_mutation_guard | runnable_frontier | Close the actual observed hole by detecting child-caused mission-truth mutation and preserving parent-only writeback. | spec review plus code correctness | yes |
| reviewer_evidence_snapshot_contract | near_frontier | Reduce future reliance on live mutable repo access by giving reviewers frozen bounded evidence artifacts. | spec/intent review | yes |
| reviewer_capability_qualification | near_frontier | Make this failure mode part of qualification/release proof. | qualification review | yes |
| platform_readonly_sandbox | deferred_or_descoped | True per-agent filesystem/tool sandboxing may require platform support outside this repo. | follow-up only | no |

## 8. Option Set

- Option A: Add more prompt text telling reviewers not to mutate mission truth.
- Option B: Add parent-owned mutation snapshots, contaminated-wave rejection, and authority checks, then reduce live context through frozen evidence snapshots.

## 9. Selected Architecture

Select Option B. The plan is a contract-centered capability boundary rather than another prompt patch. A review wave gets a parent-owned pre-review truth snapshot covering gates, closeouts, state, visible review artifacts, specs, receipts, bundles, and mission-close artifacts. Child outputs are treated as untrusted raw evidence. Before parent records any clean or non-clean review outcome, the parent validates that mission truth did not change outside an allowed parent-owned writeback phase. If contamination is detected, the parent records or surfaces a blocking review-lane violation and routes to repair/replan instead of clearing the gate. In parallel, reviewer briefs should move toward frozen evidence snapshots so child reviewers do not need direct live paths for ordinary bounded review.

## 10. Rejected Alternatives And Rationale

- Prompt-only enforcement: rejected because it already failed in live use.
- Hidden parent-chat token only: rejected because mission authority must be visible and validateable from repo artifacts, not private memory.
- External babysitter runtime: rejected because Codex1’s north star is native Codex skills and Ralph discipline, not OMX-style wrapper control.
- Immediate full platform sandboxing: deferred because the repo cannot assume platform-level permission primitives it does not expose.

## 11. Migration / Rollout / Rollback Posture

- Migration posture: additive hardening around review-loop and review outcome recording; preserve existing clean review paths when no contamination exists.
- Rollout posture: prove with targeted regression tests before expanding review UX changes.
- Rollback posture: if the guard is too strict, keep the parent-only authority invariant and narrow the watched artifact set only with explicit proof.

## 12. Review Bundle Design

- Mandatory review lenses: correctness, evidence_adequacy, interface_compatibility, operability_rollback_observability.
- Required receipts: targeted runtime tests for contaminated child wave rejection, clean parent writeback, Stop-hook parent/child behavior, and mission artifact validation.
- Required changed-file context: runtime/review-loop authority code, internal command routing, tests, skills/docs, and this mission’s spec support files.
- Mission-close claims requiring integrated judgment: child reviewer mutation is no longer silently accepted; parent-only review writeback remains usable; qualification guards against recurrence.

## 13. Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| reviewer_lane_mutation_guard | Add deterministic mutation/authority guard around child review waves and parent review outcome recording. | runnable | solo | none |
| reviewer_evidence_snapshot_contract | Add frozen reviewer evidence/brief contract to minimize live mutable repo review context. | runnable | solo | mutation guard complete |
| reviewer_capability_qualification | Add qualification proof for findings-only lane capability boundary and contaminated-wave rejection. | runnable | solo | reviewer_lane_mutation_guard, reviewer_evidence_snapshot_contract |

## 14. Execution Graph And Safe-Wave Rules

- Graph summary: sequential. The mutation guard must land before evidence snapshot ergonomics and qualification closure.
- Safe-wave rule 1: do not run reviewer evidence snapshot changes in parallel with mutation guard changes because both touch review bundle/brief semantics.
- Safe-wave rule 2: qualification changes wait until the runtime and evidence contracts are stable.

## 15. Risks And Unknowns

- Native subagents may still have filesystem/tool access; the repo-level fix must honestly detect/reject contamination if prevention is not enforceable.
- Watched artifact sets can become too broad and cause false positives unless parent-owned writeback phases are represented cleanly.
- Evidence snapshots can become too small and weaken review quality if they omit needed context.

## 16. Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-1 | Implement mutation detection and rejection before broader reviewer UX changes. | This closes the observed hole without pretending prompts are capability boundaries. | live incident, Outcome Lock | runtime, review-loop, tests | 1 |
| D-2 | Keep parent-owned review outcome as the only writeback path. | Child reviewers should return findings only. | review-lane-role-contract, Outcome Lock | record-review-outcome, review ledger, gates | 1 |
| D-3 | Use frozen evidence snapshots as a second-stage quality/safety improvement. | It reduces need for live mutable repo paths without replacing reviewer judgment. | blueprint | review bundles, docs | 1 |

## 17. Replan Policy

- Reopen Outcome Lock when: child reviewer mutation must be allowed, parent-only writeback is abandoned, or an external wrapper runtime becomes required.
- Reopen blueprint when: platform-level read-only lanes become available and materially change the route, or mutation guards prove insufficient.
- Reopen execution package when: watched artifact set, authority token model, or proof obligations change.
- Local repair allowed when: tests expose missing watched paths or overly broad false-positive detection within the same guard architecture.
