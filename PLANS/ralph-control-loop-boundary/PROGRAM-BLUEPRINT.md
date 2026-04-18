---
artifact: program-blueprint
mission_id: ralph-control-loop-boundary
version: 1
lock_revision: 1
blueprint_revision: 16
plan_level: 5
risk_floor: 4
problem_size: M
status: approved
proof_matrix:
- claim_ref: claim:conversation_yields_without_lease
  statement: Normal user-interaction Stop-hook turns yield when no active parent loop lease exists, even if another mission has open or failed gates.
  required_evidence:
  - crates/codex1-core/src/runtime.rs
  - crates/codex1/src/internal/mod.rs
  - crates/codex1/tests/runtime_internal.rs
  review_lenses:
  - correctness
  - spec_conformance
  governing_contract_refs:
  - lock:1
- claim_ref: claim:explicit_parent_loops_still_enforced
  statement: Explicit parent loop leases for plan, execute, review-loop, and autopilot still block on owed mission work.
  required_evidence:
  - crates/codex1-core/src/runtime.rs
  - .codex/skills
  review_lenses:
  - correctness
  - intent_alignment
  governing_contract_refs:
  - lock:1
- claim_ref: claim:pause_close_escape
  statement: Users have a first-class pause/close escape for Ralph continuation without editing hook files.
  required_evidence:
  - .codex/skills
  - docs/runtime-backend.md
  review_lenses:
  - spec_conformance
  - operability_rollback_observability
  governing_contract_refs:
  - lock:1
- claim_ref: claim:qualification_proves_control_boundary
  statement: Qualification proves the scoped Stop-hook control-loop boundary with hooks installed.
  required_evidence:
  - crates/codex1/src/commands/qualify.rs
  - crates/codex1/tests/qualification_cli.rs
  - docs/qualification/gates.md
  review_lenses:
  - evidence_adequacy
  - release_gate_integrity
  governing_contract_refs:
  - lock:1
- claim_ref: claim:subagents_are_ralph_exempt
  statement: All subagent Stop-hook turns yield by default and are not forced into parent gate resolution.
  required_evidence:
  - crates/codex1/src/internal/mod.rs
  - crates/codex1/tests/runtime_internal.rs
  review_lenses:
  - correctness
  - interface_compatibility
  governing_contract_refs:
  - lock:1
decision_obligations:
- obligation_id: obligation:lease-scoped-ralph
  question: Should Ralph enforcement be global or lease-scoped?
  why_it_matters: This is the core UX failure mode.
  affects:
  - architecture_boundary
  - protected_surface_risk
  governing_contract_refs:
  - lock:1
  review_contract_refs:
  - review:runtime
  mission_close_claim_refs:
  - claim:conversation_yields_without_lease
  - claim:explicit_parent_loops_still_enforced
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - OUTCOME-LOCK.md
  status: selected
  resolution_rationale: The user explicitly rejected global blocking and locked parent-loop-only enforcement.
  evidence_refs:
  - PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
- obligation_id: obligation:subagent-stop-policy
  question: Should subagents need special completion predicates to stop?
  why_it_matters: Reviewer lanes were repeatedly trapped by parent gate blockers.
  affects:
  - review_contract
  - execution_sequencing
  governing_contract_refs:
  - lock:1
  review_contract_refs:
  - review:runtime
  mission_close_claim_refs:
  - claim:subagents_are_ralph_exempt
  blockingness: critical
  candidate_route_count: 2
  required_evidence:
  - OUTCOME-LOCK.md
  status: selected
  resolution_rationale: All subagents are Ralph-exempt; parent handles missing or invalid output.
  evidence_refs:
  - PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md
  proof_spike_scope: null
  proof_spike_success_criteria: []
  proof_spike_failure_criteria: []
  proof_spike_discharge_artifacts: []
  proof_spike_failure_route: null
selected_target_ref: spec:control_loop_qualification
---
# Program Blueprint

## Locked Mission Reference

- Mission id: `ralph-control-loop-boundary`
- Lock revision: `1`
- Outcome summary: Ralph continuation is scoped to explicit parent loop workflows; normal user interaction and all subagents can stop without mission-gate blocking.

## Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | Stop hook currently has a narrow reviewer-lane bypass before normal resume handling. | `crates/codex1/src/internal/mod.rs:555` | repo | code read | current | current | high |
| T2 | verified_fact | Normal stop-hook handling routes through `resolve_resume`, so unrelated open/failed gates can block ordinary stops. | `crates/codex1-core/src/runtime.rs:5131` | repo | code read | current | current | high |
| T3 | user_fact | Ralph must govern only explicit parent loop modes, not normal user discussion or subagents. | `PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md` | lock | user-stated | locked revision 1 | current | high |
| T4 | verified_fact | Native stop-hook input has optional lane metadata but may be only `cwd` for ordinary parent turns. | `crates/codex1/src/internal/mod.rs:481` | repo | code read | current | current | high |
| T5 | verified_fact | The project hook is currently paused manually as an emergency escape. | `.codex/hooks.json.paused` | repo | filesystem state | paused hook file exists | current | high |

## System Model

- Touched surfaces: `codex1 internal stop-hook`, core Ralph stop decisions, hidden `.ralph` control state, public skills, setup/qualification docs, and runtime tests.
- Boundary summary: mission truth remains parent-owned; the new control plane only decides whether Stop hook enforcement is active for the current parent loop.
- Hidden coupling summary: Stop hook behavior currently conflates mission incompleteness with current-turn continuation authority. The route separates these by requiring an explicit parent loop lease before mission blockers become Stop-hook blockers.

## Invariants And Protected Behaviors

- Open or failed mission gates block only an active parent loop lease, never ordinary user communication.
- All subagent Stop-hook turns yield by default; parent orchestration judges missing or invalid subagent output later.
- `$plan`, `$execute`, `$review-loop`, and `$autopilot` can explicitly acquire parent loop authority so Ralph still continues those workflows.
- `$clarify` remains a manual handoff unless `$autopilot` owns the workflow.
- The emergency hook-pause workaround is replaced by a first-class pause/close control path.

## Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | With no active parent loop lease, Stop hook yields despite open/failed mission gates. | runtime stop-hook tests | `ralph_loop_lease_runtime` | yes |
| P2 | Generic subagent Stop-hook payloads yield despite open/failed mission gates. | runtime stop-hook tests | `ralph_loop_lease_runtime` | yes |
| P3 | Active parent plan/execute/review/autopilot loop leases still block on owed mission work. | runtime stop-hook tests | `ralph_loop_lease_runtime` | yes |
| P4 | Public skills describe when loop leases are acquired/released and expose a user pause/close path. | skill/doc tests | `loop_skill_surface_and_pause` | yes |
| P5 | Qualification proves the new control boundary and hook installation remains sane. | qualification CLI tests | `control_loop_qualification` | yes |

## Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Should Ralph be global or lease-scoped? | Defines the core product behavior. | critical | resolved: lease-scoped parent loops only | `OUTCOME-LOCK.md` |
| DO-2 | Should subagents need special completion predicates to stop? | Avoids recreating reviewer-specific stop bugs. | critical | resolved: no, all subagents are Ralph-exempt | `OUTCOME-LOCK.md` |
| DO-3 | Should pause/close be a public skill or equivalent control state? | Affects user-facing escape UX. | major | resolved for planning: provide a public `$close`/pause surface backed by control state unless implementation proves an equivalent better name | `OUTCOME-LOCK.md` |

## In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| `ralph_loop_lease_runtime` | runnable_frontier | Introduces the durable control-plane state and stop-hook decision boundary. | runtime correctness review | yes |
| `loop_skill_surface_and_pause` | runnable_frontier | Wires public skills to the new loop semantics and gives users a first-class escape. | spec/intent review | yes |
| `control_loop_qualification` | runnable_frontier | Proves the boundary in qualification and support docs so the hook can be safely restored. | qualification/evidence review | yes |
| Existing `review-loop-delegated-review-only` failed gate repair | deferred_or_descoped | It is a separate mission branch; this mission fixes the control loop that made that branch painful. | n/a | no |

## Option Set

- Option A: durable parent loop lease under `.ralph`, acquired by explicit loop skills and consulted by Stop hook.
- Option B: metadata-only bypasses for user turns and subagents.

## Selected Architecture

Select Option A. Add a small durable Ralph control-plane state that records whether a parent loop lease is active, paused, or absent. Stop hook first classifies the current turn. Generic subagent payloads yield immediately. Parent turns enforce mission blockers only when an active parent loop lease exists. Without a lease, Stop hook yields with passive status rather than blocking. Public loop skills acquire/release or refresh leases through internal commands; a pause/close surface clears or pauses the lease.

## Rejected Alternatives And Rationale

- Metadata-only bypasses are rejected because ordinary native parent turns may provide only `cwd`, so the runtime needs durable local state.
- Reviewer-specific exemptions are rejected because the locked product rule is all subagents are Ralph-exempt.
- Disabling `.codex/hooks.json` is rejected as normal UX because it removes all Ralph safety instead of scoping it.

## Migration / Rollout / Rollback Posture

- Migration posture: introduce the control state as absent-by-default yielding behavior; explicit loop skills opt into enforcement.
- Rollout posture: keep the hook command installed; restore `.codex/hooks.json` only after proof shows normal conversation and subagents are not trapped.
- Rollback posture: preserve manual hook pause as emergency fallback until `$close`/pause is implemented and qualified.

## Review Bundle Design

- Mandatory review lenses: correctness, spec_conformance, interface_compatibility, evidence_adequacy, operability_rollback_observability.
- Required receipts: runtime stop-hook tests, full `runtime_internal` subset, qualification CLI tests for control-loop behavior, `cargo fmt --all --check`, mission artifact validation.
- Required changed-file context: stop-hook code, runtime control-plane code, paths/lib exports, public skills, docs, qualification tests.
- Mission-close claims requiring integrated judgment: explicit parent loops still continue, normal user discussion yields, all subagents yield, and pause/close avoids manual hook edits.

## Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| `ralph_loop_lease_runtime` | Implement durable loop lease state and core Stop-hook semantics. | runnable | solo | lock:1 |
| `loop_skill_surface_and_pause` | Update public skills and pause/close UX to use the new loop lease contract. | runnable | solo | `ralph_loop_lease_runtime` |
| `control_loop_qualification` | Add qualification/support proof that the control boundary works with hooks installed. | runnable | solo | `ralph_loop_lease_runtime`, `loop_skill_surface_and_pause` |

## Execution Graph And Safe-Wave Rules

- Graph summary: sequential three-node graph. Runtime lease semantics must land first; skill UX and qualification depend on its command/API shape.
- Safe-wave rule 1: do not run skill surface and runtime implementation in parallel because command names and state shape are shared.
- Safe-wave rule 2: do not run qualification before skill/runtime semantics are stable because proof rows must exercise the final public contract.

## Risks And Unknowns

- The exact native Stop-hook payload for normal user turns may not include enough metadata; lease absence must therefore be safe and intentional.
- Explicit loop skills are instruction-driven, so skill docs and backend commands must make lease acquisition hard to skip.
- Restoring hook installation before the boundary is qualified could reintroduce the user-blocking loop.

## Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-1 | Use a durable parent loop lease rather than global Stop-hook enforcement. | It is the smallest architecture that distinguishes explicit autonomous loops from conversation when hook input is sparse. | `OUTCOME-LOCK.md`, `crates/codex1/src/internal/mod.rs:481` | runtime, skills, docs | 1 |
| D-2 | Treat all subagents as Ralph-exempt. | The parent owns integration and should handle missing/partial subagent output; subagents should not be trapped by mission gates. | `OUTCOME-LOCK.md` | stop-hook runtime, internal orchestration docs | 1 |
| D-3 | Plan a first-class pause/close surface. | The manual hook move proved a real need for user-controlled loop suspension. | `.codex/hooks.json.paused`, `OUTCOME-LOCK.md` | skills, setup docs, runtime | 1 |

## Replan Policy

- Reopen Outcome Lock when: the user changes the rule that only explicit parent loops are Ralph-governed.
- Reopen blueprint when: native hook payload limitations force a different control-plane architecture or public UX.
- Reopen execution package when: command/state shape or write scope changes after packaging.
- Local repair allowed when: fixes stay inside the selected spec's runtime command and test scope without changing public loop semantics.
