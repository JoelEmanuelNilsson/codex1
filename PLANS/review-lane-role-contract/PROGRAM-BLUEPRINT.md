---
artifact: program-blueprint
mission_id: review-lane-role-contract
version: 1
lock_revision: 1
blueprint_revision: 10
plan_level: 5
risk_floor: 5
problem_size: L
status: approved
proof_matrix:
- claim_ref: claim:findings_only_reviewer_profiles
  statement: Reviewer lanes are findings-only roles with model routing, severity threshold, output schema, and six-loop cap semantics.
  required_evidence:
  - .codex/skills/review-loop/SKILL.md
  - docs/MULTI-AGENT-V2-GUIDE.md
  - docs/runtime-backend.md
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - lock:1
  - review_profiles
- claim_ref: claim:ralph_child_review_isolation
  statement: Child review lanes can deliver findings without parent-gate deadlock and cannot own mission-truth writeback.
  required_evidence:
  - crates/codex1-core/src/ralph.rs
  - crates/codex1-core/src/runtime.rs
  - crates/codex1/tests/runtime_internal.rs
  review_lenses:
  - correctness
  - operability_rollback_observability
  - evidence_adequacy
  governing_contract_refs:
  - ralph
  - multi_agent
- claim_ref: claim:review_loop_orchestration
  statement: $review-loop runs bounded waves, aggregates P0/P1/P2 findings, reruns targeted repairs, and routes to replan after six non-clean loops.
  required_evidence:
  - .codex/skills/review-loop/SKILL.md
  - docs/runtime-backend.md
  - crates/codex1/src/commands/qualify.rs
  review_lenses:
  - spec_conformance
  - correctness
  - evidence_adequacy
  governing_contract_refs:
  - review_loop
  - qualification
- claim_ref: claim:review_loop_skill_surface
  statement: $review-loop is canonical and $review is no longer a public skill name.
  required_evidence:
  - .codex/skills/review-loop/SKILL.md
  - crates/codex1/src/support_surface.rs
  - crates/codex1/tests/qualification_cli.rs
  review_lenses:
  - spec_conformance
  - correctness
  - interface_compatibility
  - evidence_adequacy
  governing_contract_refs:
  - lock:1
  - skill_surface
decision_obligations: []
selected_target_ref: mission:review-lane-role-contract
---
# Program Blueprint

## 1. Locked Mission Reference

- Mission id: `review-lane-role-contract`
- Lock revision: `1`
- Outcome summary: `$review-loop` is the canonical parent/orchestrator review workflow, `$review` is removed, child reviewers are findings-only prompt roles, model routing is explicit, review loops cap at six, and Ralph-safe child-lane behavior prevents deadlock or mission-truth mutation.

## 2. Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | The lock requires `$review-loop` and removal of `$review`. | `OUTCOME-LOCK.md` | user_lock | ratified lock | revision 1 | current | high |
| T2 | verified_fact | Current `$review` mixes parent orchestration, reviewer judgment, and writeback. | `.codex/skills/review/SKILL.md` | repo | file read | current | current | high |
| T3 | verified_fact | Current internal orchestration already says parent owns mission truth and child agents do bounded work. | `.codex/skills/internal-orchestration/SKILL.md` | repo | file read | current | current | high |
| T4 | verified_fact | Support surface currently names `review` in managed skills/scaffolds/tests. | `crates/codex1/src/support_surface.rs` | repo | source search | current | current | high |

## 3. System Model

- Touched surfaces: `.codex/skills`, support-surface setup/doctor/qualification code, `crates/codex1-core/src/{ralph.rs,runtime.rs}`, runtime/qualification docs, and tests.
- Boundary summary: parent `$review-loop` orchestrates review waves and owns writeback; child reviewers only return `NONE` or findings.
- Hidden coupling summary: skill names are copied/linked by setup, validated by doctor/qualification, mentioned in AGENTS/docs, and used by user/Ralph workflows.

## 4. Invariants And Protected Behaviors

- `$review` must not remain a public skill name.
- Child reviewers must not call `$review-loop`, clear gates, write mission artifacts, or decide completion.
- Six consecutive non-clean loops route to replan.
- P0/P1/P2 means not clean; P3 is non-blocking by default.
- Code-producing work receives `gpt-5.3-codex` bug/correctness review at proof-worthy checkpoints.
- Spec/intent, integration, PRD, and mission-close judgment use `gpt-5.4`.

## 5. Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | Skill/support surfaces expose `$review-loop` and no longer require or advertise `$review`. | source and qualification tests | review_loop_skill_surface | yes |
| P2 | Reviewer profiles encode model routing, output schema, severity threshold, and six-loop semantics. | skill/docs/tests | reviewer_profile_contracts | yes |
| P3 | Child review lanes can return findings without parent gate deadlock and cannot own mission-truth writeback. | runtime tests | ralph_review_lane_isolation | yes |
| P4 | `$review-loop` orchestrates clean, repair/re-review, and six-loop replan paths. | workflow/qualification tests | review_loop_orchestration | yes |

## 6. Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Preserve `$review`? | Naming confusion caused the mission. | critical | resolved: remove `$review` | `OUTCOME-LOCK.md` |
| DO-2 | Are child reviewers skill users? | Decides writeback authority. | critical | resolved: no, findings-only prompt roles | `OUTCOME-LOCK.md` |
| DO-3 | Who chooses enforcement mechanics? | Decides planning scope. | major | resolved: planning chooses mechanics while preserving behavior | `OUTCOME-LOCK.md` |

## 7. In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| review_loop_skill_surface | completed | Rename/split public skill and support scaffolds. | spec review | yes |
| reviewer_profile_contracts | completed | Define review profiles, child schema, severity, six-loop state. | spec review | yes |
| ralph_review_lane_isolation | completed | Prevent child reviewer deadlock and child-owned writeback. | runtime review | yes |
| review_loop_orchestration | completed | Implement parent loop behavior and proof. | integration review | yes |

## 8. Option Set

- Option A: prompt-only rename and documentation.
- Option B: contract-plus-runtime route with skill rename, profile contracts, Ralph-safe child-lane support, and qualification.

## 9. Selected Architecture

Select Option B. The observed failure proves prompt-only discipline is insufficient. The route is sequential: first migrate the public skill/support surface to `$review-loop`, then define reviewer profiles and output contracts, then add Ralph/runtime child-lane isolation, then implement full parent `$review-loop` orchestration with the six-loop cap and replan route. Only the first slice is runnable now because later slices depend on the renamed public surface.

## 10. Rejected Alternatives And Rationale

- Keep `$review` as alias: rejected by the lock.
- Prompt-only child reviewer discipline: rejected because child/parent blocking and unintended mutation already happened.
- One generic review profile: rejected because local/spec, integration, and mission-close require separate defaults.

## 11. Migration / Rollout / Rollback Posture

- Migration posture: remove `.codex/skills/review`, add `.codex/skills/review-loop`, and update support-surface and qualification expectations together.
- Rollout posture: repo-native skills and deterministic backend only; no wrapper runtime.
- Rollback posture: repair via support-surface tests and manifest-backed setup/restore if skill migration breaks managed surfaces.

## 12. Review Bundle Design

- Mandatory review lenses: spec_conformance, correctness, interface_compatibility, evidence_adequacy, operability_rollback_observability.
- Required receipts: skill/support tests, runtime/Ralph tests, qualification tests, and review-loop proof receipts.
- Required changed-file context: public skills, support-surface registry/scaffold, runtime-backend and multi-agent docs, runtime child-lane code, qualification docs/tests.
- Mission-close claims requiring integrated judgment: `$review-loop` canonical, `$review` removed, child reviewers findings-only, no child deadlock, no child writeback, six-loop cap routes to replan.

## 13. Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| review_loop_skill_surface | Rename the public review skill surface and support scaffolds. | complete | solo | none |
| reviewer_profile_contracts | Define review profiles, severity, model routing, and child output schema. | runnable | solo | review_loop_skill_surface |
| ralph_review_lane_isolation | Add runtime/Ralph protection for child review lanes. | complete | solo | reviewer_profile_contracts (already satisfied) |
| review_loop_orchestration | Implement and prove parent review-loop behavior. | complete | solo | ralph_review_lane_isolation (already satisfied) |

## 14. Execution Graph And Safe-Wave Rules

- Graph summary: prior review-loop skill, profile, and Ralph isolation slices are complete and reviewed; all planned workstreams are complete and the mission is ready for mission-close review. Prior completed slices are satisfied route history, so the current execution package stays bounded to parent review-loop orchestration.
- Safe-wave rule 1: serialize writes across `.codex/skills`, support-surface scaffolding, and qualification expectations because the skill rename is globally coupled.
- Safe-wave rule 2: runtime/Ralph enforcement must wait for profile contracts so tests can assert concrete lane roles and output shapes.

## 15. Risks And Unknowns

- Removing `$review` may break support-surface qualification unless every managed skill list and scaffold is updated.
- Native Codex may not expose hard child sandboxing; if so, prove parent-only writeback and no-deadlock behavior within native limits.
- Existing `codex1_review` config points to `gpt-5.4-mini`, which may need renaming/reinterpretation because blocking review defaults now favor `gpt-5.4` and `gpt-5.3-codex`.

## 16. Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-1 | `$review-loop` is canonical and `$review` is removed. | Prevent reviewer/orchestrator confusion. | `OUTCOME-LOCK.md` | skills, support surface, docs, tests | 1 |
| D-2 | Child reviewers are findings-only prompt roles. | Parent owns mission truth and writeback. | `OUTCOME-LOCK.md`, `internal-orchestration` | review loop, runtime/Ralph | 1 |
| D-3 | Six non-clean loops route to replan. | Persistent findings indicate route/contract failure. | `OUTCOME-LOCK.md` | review loop, replan policy | 1 |
| D-4 | Review profiles are scope-specific. | Different review scopes need different judgment shapes. | `OUTCOME-LOCK.md` | profile contract, review loop | 1 |

## 17. Replan Policy

- Reopen Outcome Lock when: preserving `$review`, allowing child reviewers to mutate mission truth, changing six-loop cap, or lowering P0/P1/P2 blocking semantics becomes necessary.
- Reopen blueprint when: native Codex constraints require a different architecture for child-lane isolation or review-loop orchestration.
- Reopen execution package when: write scopes or dependency order change without changing route.
- Local repair allowed when: wording, tests, or implementation details need tightening while preserving locked behavior.
