---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: review_lane_completion_guard
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 8
blueprint_fingerprint: sha256:ac8b3f655d713f33eb59ab49b99a5d3cb78fd5b25abfc22c89e55c9cc1c28b04
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

Enforce required reviewer-output lane coverage before a parent review-loop can record a clean review outcome.

## In Scope

- Add a machine-readable review-lane completion contract for blocking review bundles or review outcomes.
- Ensure clean `record-review-outcome` rejects missing required reviewer-output coverage for profiles implied by the bundle.
- For code-producing slices that require correctness review, require at least one durable code/correctness reviewer-output before clean review can pass.
- Preserve the parent-owned writeback model: reviewers may write only reviewer-output artifacts, and only the parent records review outcomes.
- Add regression tests that reproduce contradiction `e7198761-0b56-4978-b587-2aeaa944a03e`: one spec/intent reviewer-output alone must not clear a code-producing review boundary.
- Update qualification or review-loop documentation so missing required lane output routes to repair/replan/blocked review, not clean.

## Out Of Scope

- Solving native sub-agent liveness or model nonresponse generally.
- Changing the locked setup/init installer behavior.
- Mutating real user home during tests.
- Replacing reviewer-agent orchestration with parent self-review.

## Dependencies

- Locked Outcome Lock revision 1.
- Program Blueprint revision 8.
- `doctor_restore_verification` implementation and proof are preserved but its clean review event is invalidated as sufficient review truth by contradiction `e7198761-0b56-4978-b587-2aeaa944a03e`.

## Touched Surfaces

- Runtime review bundle/outcome contracts.
- Reviewer-output evidence validation.
- Review-loop skill/documentation.
- Runtime or qualification tests.

## Read Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/runtime_internal.rs
- crates/codex1/tests/qualification_cli.rs
- .codex/skills/review-loop/SKILL.md
- PLANS/global-codex1-installer/REPLAN-LOG.md
- PLANS/global-codex1-installer/REVIEW-LEDGER.md

## Write Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/runtime_internal.rs
- crates/codex1/tests/qualification_cli.rs
- .codex/skills/review-loop/SKILL.md
- PLANS/global-codex1-installer/specs/review_lane_completion_guard/SPEC.md
- PLANS/global-codex1-installer/specs/review_lane_completion_guard/REVIEW.md
- PLANS/global-codex1-installer/specs/review_lane_completion_guard/NOTES.md
- PLANS/global-codex1-installer/specs/review_lane_completion_guard/RECEIPTS/README.md
- PLANS/global-codex1-installer/specs/review_lane_completion_guard/RECEIPTS/2026-04-17-review-lane-completion-guard-proof.md

## Interfaces And Contracts Touched

- Review bundle contract.
- Review outcome writeback validation.
- Reviewer-output evidence refs.
- Review-loop public skill contract.
- Qualification gate coverage for delegated review.

## Implementation Shape

Represent required lane coverage in a deterministic way that can be validated without trusting parent prose. The minimal acceptable approach is to derive required coverage from bundle kind, target type, mandatory lenses, and changed files, then require matching persisted reviewer-output artifacts for clean review outcomes.

For this mission, the critical rule is:

- If a spec-review bundle includes `correctness` and reviews code-producing changed files, a clean outcome must cite at least one reviewer-output artifact from a code/correctness reviewer lane and at least one reviewer-output artifact from the spec/intent or proof lane that judged the spec contract.

The implementation may add explicit reviewer profile fields if that is cleaner, but it must preserve compatibility or provide a clear migration path for existing bundles/tests.

## Proof-Of-Completion Expectations

- Runtime regression test rejects a clean review outcome that cites only a spec/intent reviewer-output for a code-producing correctness bundle.
- Runtime regression test accepts a clean review outcome when required code/correctness and spec/intent reviewer-output artifacts are both present and clean.
- Existing delegated review authority, reviewer-output inbox, and qualification CLI tests still pass.
- `cargo fmt --all --check` passes.
- `cargo check -p codex1` passes.

## Non-Breakage Expectations

- Review outcomes with P0/P1/P2 reviewer findings still cannot be recorded as clean.
- Contaminated review waves can still route to replan without fake reviewer-output refs.
- Existing historical review artifacts remain readable.

## Review Lenses

- correctness
- review honesty
- evidence adequacy
- backward compatibility
- qualification adequacy

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

- `contradiction:e7198761-0b56-4978-b587-2aeaa944a03e`
- `.ralph/missions/global-codex1-installer/reviewer-outputs/666382e9-0198-4d6c-b007-2a334aafe3f2/4a5d6937-da75-4fd3-bd2d-ad8f01d15dde.json`
- `PLANS/global-codex1-installer/REPLAN-LOG.md`

## Freshness Notes

- Based on contradiction `e7198761-0b56-4978-b587-2aeaa944a03e` recorded on 2026-04-17.
- Re-read `runtime.rs` review writeback validation before implementation because this repo has active unrelated mission changes.
- The installer implementation and tests are preserved; this spec reopens review sufficiency, not the setup/init product contract.

## Support Files

- `REVIEW.md` records per-spec review context and dispositions.
- `NOTES.md` holds non-authoritative local notes or spike results.
- `RECEIPTS/` stores proof receipts that support completion claims.
