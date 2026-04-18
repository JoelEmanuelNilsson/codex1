---
artifact: program-blueprint
mission_id: manual-clarify-handoff-boundary
version: 1
lock_revision: 1
blueprint_revision: 1
plan_level: 5
risk_floor: 4
problem_size: M
status: approved
proof_matrix:
- claim_ref: claim:autopilot_continuation_preserved
  statement: Autopilot remains the workflow that continues from clarify into planning automatically.
  required_evidence:
  - .codex/skills/autopilot/SKILL.md
  - crates/codex1/tests/runtime_internal.rs
  review_lenses:
  - spec_conformance
  - correctness
  governing_contract_refs:
  - lock:1
  - autopilot
- claim_ref: claim:manual_clarify_handoff
  statement: Manual ratified clarify yields for explicit plan invocation rather than blocking Stop with auto-plan.
  required_evidence:
  - crates/codex1-core/src/runtime.rs
  - crates/codex1/tests/runtime_internal.rs
  review_lenses:
  - spec_conformance
  - correctness
  - interface_compatibility
  governing_contract_refs:
  - lock:1
  - stop_hook
decision_obligations: []
selected_target_ref: spec:manual_clarify_handoff_runtime
---
# Program Blueprint

## Locked Mission Reference

- Mission id: `manual-clarify-handoff-boundary`
- Lock revision: `1`
- Outcome summary: manual `$clarify` must stop at a clean user handoff after lock ratification, while `$autopilot` remains the workflow that continues into `$plan` automatically.

## Truth Register Summary

| Row | Type | Statement | Evidence ref | Source type | Observation basis | Observed revision or state | Freshness | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | verified_fact | Current locked clarify emits actionable planning continuation. | `crates/codex1-core/src/runtime.rs` | repo | code read | `ResumeMode::Continue`, `next_phase = planning` | current | high |
| T2 | verified_fact | Stop-hook blocks actionable non-terminal states. | `crates/codex1-core/src/runtime.rs` | repo | code read | `ActionableNonTerminal => block` | current | high |
| T3 | user_fact | Manual clarify should wait for explicit `$plan`; autopilot should continue. | `OUTCOME-LOCK.md` | lock | user-stated | locked | current | high |

## System Model

- Touched surfaces: `crates/codex1-core/src/runtime.rs`, `crates/codex1/tests/runtime_internal.rs`, `$clarify`, `$autopilot`, and `docs/runtime-backend.md`.
- Boundary summary: manual skill handoff is represented as durable waiting/yield state; autopilot is responsible for consuming that boundary and continuing to planning.
- Hidden coupling summary: `init-mission` closeout shape controls Stop-hook behavior; changing it affects all ratified clarify bootstrap flows and planning tests.

## Invariants And Protected Behaviors

- Manual `$clarify` must not auto-start `$plan` via Stop-hook blocking.
- `$autopilot` must still be allowed to continue through clarify -> plan.
- Manual users must still be able to invoke `$plan` explicitly after clarify.
- Waiting state must be durable and artifact-backed, not chat-only.

## Proof Matrix

| Proof row | What must be proven | Evidence class | Owner | Blocking |
| --- | --- | --- | --- | --- |
| P1 | Locked manual clarify yields without Stop-hook block and asks for explicit `$plan`. | runtime integration test | manual_clarify_handoff_runtime | yes |
| P2 | Autopilot documentation/contract still says autopilot consumes clarify -> plan. | skill/doc assertion | manual_clarify_handoff_runtime | yes |
| P3 | Existing mission artifact, gate, and runtime tests remain green. | test suite | manual_clarify_handoff_runtime | yes |

## Decision Obligations

| Obligation id | Question | Why it matters | Blockingness | Status | Evidence refs |
| --- | --- | --- | --- | --- | --- |
| DO-1 | Should manual clarify or autopilot own automatic planning? | Defines user control boundary. | critical | resolved: autopilot only | `OUTCOME-LOCK.md` |

## In-Scope Work Inventory

| Work item | Class | Why it exists | Proof / review owner | Finish in this mission? |
| --- | --- | --- | --- | --- |
| manual_clarify_handoff_runtime | runnable_frontier | Fix runtime/state/Stop-hook behavior after ratified manual clarify. | spec review | yes |

## Selected Architecture

Use a durable waiting handoff for locked manual clarify. The ratified lock closeout should use `needs_user` / `yield_to_user` semantics with `next_phase = planning` and a canonical request instructing the user to invoke `$plan` manually, unless `$autopilot` owns the workflow. This keeps manual skill boundaries under user control while preserving autopilot as the explicit continuation router.

## Rejected Alternatives And Rationale

- Keep current `continue_required` planning handoff: rejected because it causes Stop-hook to block and start planning during manual clarify.
- Make Stop-hook special-case all planning handoffs: rejected because planning/package execution still needs actionable blocking in non-clarify contexts.
- Use hidden chat memory to know whether autopilot was invoked: rejected because Codex1 mission truth must be durable.

## Review Bundle Design

- Mandatory review lenses: spec_conformance, correctness, interface_compatibility, evidence_adequacy.
- Required receipts: targeted stop-hook/manual clarify tests, autopilot contract assertion, artifact validation.
- Required changed-file context: runtime closeout/state logic, runtime integration tests, clarify/autopilot docs if touched.
- Mission-close claims requiring integrated judgment: manual clarify handoff is non-blocking; autopilot continuation remains intact.

## Workstream Overview

| Spec id | Purpose | Packetization status | Owner mode | Depends on |
| --- | --- | --- | --- | --- |
| manual_clarify_handoff_runtime | Fix runtime handoff after ratified manual clarify. | runnable | solo | none |

## Execution Graph And Safe-Wave Rules

- Single runnable runtime slice; no parallel wave.
- Keep writes localized to runtime/tests/docs unless proof exposes a needed adjacent update.

## Risks And Unknowns

- Some existing tests may assume ratified `init-mission` immediately enters actionable planning; update only the tests that model manual clarify.
- Autopilot may need a later explicit consume-waiting-handoff helper, but this slice can preserve its contract if no current autopilot backend implementation depends on the old closeout shape.

## Decision Log

| Decision id | Statement | Rationale | Evidence refs | Affected artifacts | Adopted in revision |
| --- | --- | --- | --- | --- | --- |
| D-1 | Manual clarify yields at the planning boundary. | User explicitly wants manual `$plan` invocation. | `OUTCOME-LOCK.md` | runtime, tests | 1 |
| D-2 | Autopilot owns automatic continuation across clarify -> plan. | `$autopilot` is the end-to-end workflow. | `OUTCOME-LOCK.md`, `.codex/skills/autopilot/SKILL.md` | autopilot contract | 1 |

## Replan Policy

- Reopen lock if manual clarify must auto-plan or autopilot must stop after clarify.
- Reopen blueprint if native runtime cannot represent manual waiting handoff durably.
- Local repair allowed for tests/docs/runtime mechanics that preserve the locked behavior.
