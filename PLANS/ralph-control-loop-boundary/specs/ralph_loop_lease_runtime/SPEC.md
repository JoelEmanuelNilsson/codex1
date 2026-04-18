---
artifact: workstream-spec
mission_id: ralph-control-loop-boundary
spec_id: ralph_loop_lease_runtime
version: 1
spec_revision: 2
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 16
blueprint_fingerprint: sha256:2b2cb026e80b79e6dbe64eb59ad439d0fc8b42b784f76617fef2e6ca48e790a8
spec_fingerprint: null
replan_boundary:
  local_repair_allowed: false
  trigger_matrix:
  - trigger_code: write_scope_expansion
    reopen_layer: execution_package
  - trigger_code: interface_contract_change
    reopen_layer: blueprint
  - trigger_code: dependency_truth_change
    reopen_layer: execution_package
  - trigger_code: proof_obligation_change
    reopen_layer: blueprint
  - trigger_code: review_contract_change
    reopen_layer: blueprint
  - trigger_code: protected_surface_change
    reopen_layer: mission_lock
  - trigger_code: migration_rollout_change
    reopen_layer: blueprint
  - trigger_code: outcome_lock_change
    reopen_layer: mission_lock
---
# Workstream Spec

## Purpose

Implement the runtime control-plane boundary that makes Ralph Stop-hook enforcement lease-scoped: all subagents yield, ordinary parent conversation yields when no explicit loop lease is active, and active parent loop leases still enforce mission blockers.

## In Scope

- Add a durable Ralph control state path and schema for parent loop leases, including absent, active, and paused/closed states.
- Add internal commands to begin, pause/close, inspect, and clear parent loop leases, with mission id, mode, owner, and provenance.
- Update codex1 internal stop-hook to classify generic subagent turns before resume handling and to consult the parent loop lease before blocking parent turns.
- Preserve existing findings-only reviewer behavior as a subset of the generic subagent exemption.
- Add runtime tests for no-lease parent yield, generic subagent yield, active parent loop blocking, paused lease yield, and existing waiting/selection behavior.

## Out Of Scope

- Updating all public skill prose beyond the minimal command names needed by tests.
- Adding qualification gates or setup docs beyond runtime test fixtures.
- Repairing unrelated failed review gates in other missions.

## Dependencies

- Outcome Lock revision 1 is locked.
- Current stop-hook code and runtime tests are readable.

## Touched Surfaces

- crates/codex1-core/src/runtime.rs
- crates/codex1-core/src/paths.rs
- crates/codex1-core/src/lib.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- docs/runtime-backend.md

## Read Scope

- crates/codex1-core/src
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- docs/runtime-backend.md
- .codex/skills
- PLANS/ralph-control-loop-boundary
- .ralph/missions/ralph-control-loop-boundary

## Write Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1-core/src/paths.rs
- crates/codex1-core/src/lib.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- docs/runtime-backend.md
- PLANS/ralph-control-loop-boundary/specs/ralph_loop_lease_runtime

## Interfaces And Contracts Touched

- Stop-hook input classification.
- Ralph control state schema and path helpers.
- Internal loop lease commands.
- Core Stop-hook blocking/yield decision.

## Implementation Shape

Introduce a small control state under .ralph or an equivalent hidden runtime path. The default missing state is no active lease, which makes parent Stop-hook turns yield. Explicit loop commands create an active parent lease for planning_loop, execution_loop, review_loop, or autopilot_loop; while active, Stop hook delegates to existing resume/blocking logic. Generic child/subagent payloads yield before lease checks. Pause/close clears or pauses the lease so user discussion can proceed.

## Proof-Of-Completion Expectations

- cargo test -p codex1 --test runtime_internal ralph_control_loop_boundary --quiet
- cargo test -p codex1 --test runtime_internal findings_only_reviewer_stop_hook_bypasses_parent_review_gate_block --quiet
- cargo test -p codex1 --test runtime_internal manual_ratified_clarify_yields_for_explicit_plan_instead_of_blocking --quiet
- cargo fmt --all --check
- codex1 internal validate-mission-artifacts --mission-id ralph-control-loop-boundary

## Non-Breakage Expectations

- Explicit active parent loop leases still block on owed review/repair/replan work.
- Waiting/needs-user states still yield honestly.
- Malformed selection state still blocks as repair-required.
- Existing reviewer-lane metadata remains accepted but is no longer the only subagent escape.

## Review Lenses

- correctness
- spec_conformance
- interface_compatibility
- operability_rollback_observability
- evidence_adequacy

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| hook_payload_insufficient_for_subagent_detection | blueprint |
| lease_schema_expands_beyond_runtime_scope | execution_package |
| public_skill_semantics_change | blueprint |
| proof_obligation_change | blueprint |
| protected_surface_change | mission_lock |

## Truth Basis Refs

- PLANS/ralph-control-loop-boundary/OUTCOME-LOCK.md
- crates/codex1/src/internal/mod.rs:481
- crates/codex1-core/src/runtime.rs:5131


## Freshness Notes

- Refresh this spec if native Codex Stop-hook payloads add or remove reliable parent/subagent/user-turn metadata.
- Refresh this spec if the public skill loop entry model changes before execution.

## Support Files

- REVIEW.md records delegated review outcomes for this spec.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output and command transcripts.
