---
artifact: workstream-spec
mission_id: review-loop-delegated-review-only
spec_id: delegated_review_authority_contract
version: 1
spec_revision: 2
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 16
blueprint_fingerprint: sha256:cded0d54c8006846633e31bb172ffa0ba686ccbc69a147d8b1cdca40528173ca
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

Make delegated reviewer-agent judgment the explicit and machine-checkable authority for review outcomes. This means $review-loop is orchestration-only for the parent and all substantive review verdicts must come from reviewer-agent outputs.

## In Scope

- Update $review-loop and internal-orchestration language so parent/orchestrator review judgment is forbidden with no exception for small, obvious, high-context, or reviewer-system changes.
- Update runtime/backend docs so parent responsibilities are limited to orchestration, evidence preparation, aggregation, contamination detection, routing, and writeback.
- Add or tighten review outcome input/recording validation so clean or finding-bearing outcomes must cite reviewer-agent output evidence refs, while contaminated/invalid review waves route without clearing the gate.
- Preserve existing review truth snapshot and child no-mutation guards.
- Add targeted tests for missing reviewer-agent evidence rejection and valid reviewer-output acceptance.

## Out Of Scope

- Renaming $review-loop again.
- Giving child reviewers permission to clear gates or write mission artifacts.
- Building platform-level filesystem sandboxing for child agents.
- Redesigning reviewer model/profile routing beyond wording needed for this authority boundary.

## Dependencies

- Outcome Lock revision 1 for review-loop-delegated-review-only.
- Completed reviewer-lane capability boundary that already guards child mutation and keeps parent-owned writeback.
- Existing $review-loop profile and six-loop cap semantics.

## Touched Surfaces

- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- docs/MULTI-AGENT-V2-GUIDE.md
- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- PLANS/review-loop-delegated-review-only/specs/delegated_review_authority_contract

## Read Scope

- .codex/skills
- docs
- crates/codex1-core/src
- crates/codex1/src
- crates/codex1/tests
- PLANS/reviewer-lane-capability-boundary
- PLANS/review-lane-role-contract

## Write Scope

- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- docs/MULTI-AGENT-V2-GUIDE.md
- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/tests/runtime_internal.rs
- PLANS/review-loop-delegated-review-only/specs/delegated_review_authority_contract

## Interfaces And Contracts Touched

- $review-loop parent/orchestrator authority boundary.
- Child reviewer findings-only contract.
- record-review-outcome / ReviewResultInput evidence acceptance contract.
- Review gate clearing/failing semantics.
- Review truth snapshot contamination rejection path.

## Implementation Shape

First tighten the public workflow text: the parent can inspect and synthesize for orchestration, but cannot provide the substantive review verdict. Then make durable review outcome recording require explicit reviewer-agent output evidence refs for clean or finding-bearing outcomes. The accepted evidence shape should be repo-native and practical, such as refs to captured child output artifacts, review evidence snapshots, bundle notes, or recorded reviewer transcript excerpts that identify the reviewer lane. If those refs are absent, the outcome must not be able to clear or fail the target as though reviewed; it should be rejected or route as invalid/contaminated/replan depending on the existing branch model. Keep child no-mutation rules and parent-owned writeback intact.

## Proof-Of-Completion Expectations

- Test proves a parent-only clean review outcome without reviewer-agent output evidence is rejected or cannot clear the review gate.
- Test proves a clean parent-owned writeback with reviewer-agent output evidence and a clean review truth snapshot succeeds.
- Test proves a blocking finding outcome must also cite reviewer-agent evidence rather than parent-only judgment.
- Source assertions prove $review-loop, internal-orchestration, and runtime docs forbid parent self-review and preserve allowed parent orchestration responsibilities.
- cargo test -p codex1 --test runtime_internal delegated_review_authority --quiet or an equivalent targeted test passes.
- cargo run -p codex1 -- internal validate-mission-artifacts --mission-id review-loop-delegated-review-only passes after receipt writeback.

## Non-Breakage Expectations

- Existing reviewer truth snapshot and evidence snapshot tests remain green.
- Existing review-loop six-loop/replan semantics remain unchanged.
- Child reviewers still cannot mutate gates, closeouts, state, ledgers, specs, receipts, bundles, or mission-close artifacts.
- Mission-close review still uses delegated reviewer agents and parent-owned aggregation/writeback.

## Review Lenses

- correctness
- spec_conformance
- evidence_adequacy
- interface_compatibility
- operability_rollback_observability

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| write_scope_expansion | execution_package |
| interface_contract_change | blueprint |
| dependency_truth_change | execution_package |
| proof_obligation_change | blueprint |
| review_contract_change | blueprint |
| protected_surface_change | mission_lock |
| migration_rollout_change | blueprint |
| outcome_lock_change | mission_lock |

## Truth Basis Refs

- PLANS/review-loop-delegated-review-only/OUTCOME-LOCK.md
- .codex/skills/review-loop/SKILL.md
- docs/runtime-backend.md
- PLANS/reviewer-lane-capability-boundary/OUTCOME-LOCK.md

## Freshness Notes

- Current for the live bug report where parent local review was used despite intended delegated-review-only semantics.
- Current against the dirty tree after reviewer-lane-capability-boundary completion.

## Support Files

- REVIEW.md records delegated reviewer-agent review dispositions.
- NOTES.md records non-authoritative implementation notes.
- RECEIPTS/ stores proof output.
