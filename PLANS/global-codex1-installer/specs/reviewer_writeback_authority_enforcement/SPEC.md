---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: reviewer_writeback_authority_enforcement
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 9
blueprint_fingerprint: sha256:93b7d201928fb9931ce297e2a7ee5f8de2be02f13c7cce252efc697eb01561be
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

Make parent-owned review writeback authority unavailable to findings-only reviewer lanes.

## In Scope

- Prevent reviewer lanes from minting parent writeback authority by calling `capture-review-truth-snapshot`.
- Prevent reviewer lanes from using `record-review-outcome` even if they spoof the reviewer name, hold a structurally valid truth snapshot, or cite valid reviewer-output artifacts.
- Preserve bounded reviewer-output writes through `record-reviewer-output`.
- Add runtime and qualification regressions that reproduce contradiction `0d8f3968-2f41-460d-9ab8-3c08660c3aac`.
- Update review-loop/internal-orchestration docs if the runtime contract needs a new caller-context or capability field.
- Preserve mission-close review requirements; only the contaminated mission-close completion is invalidated.

## Out Of Scope

- Redesigning native Codex sub-agent APIs.
- Removing reviewer lanes or allowing parent self-review.
- Changing installer setup/init behavior.
- Publishing/release packaging.

## Dependencies

- Locked Outcome Lock revision 1.
- Program Blueprint revision 9.
- `review_lane_completion_guard` remains preserved and reviewed clean.
- Contradiction `0d8f3968-2f41-460d-9ab8-3c08660c3aac` is the governing replan cause.

## Touched Surfaces

- Review truth snapshot capture.
- Review outcome writeback validation.
- Reviewer-output inbox validation.
- Runtime/qualification tests.
- Review-loop and internal orchestration documentation if needed.

## Read Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/runtime_internal.rs
- crates/codex1/tests/qualification_cli.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/global-codex1-installer/REPLAN-LOG.md
- PLANS/global-codex1-installer/REVIEW-LEDGER.md

## Write Scope

- crates/codex1-core/src/runtime.rs
- crates/codex1/src/internal/mod.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/runtime_internal.rs
- crates/codex1/tests/qualification_cli.rs
- .codex/skills/review-loop/SKILL.md
- .codex/skills/internal-orchestration/SKILL.md
- docs/runtime-backend.md
- PLANS/global-codex1-installer/specs/reviewer_writeback_authority_enforcement/SPEC.md
- PLANS/global-codex1-installer/specs/reviewer_writeback_authority_enforcement/REVIEW.md
- PLANS/global-codex1-installer/specs/reviewer_writeback_authority_enforcement/NOTES.md
- PLANS/global-codex1-installer/specs/reviewer_writeback_authority_enforcement/RECEIPTS/README.md
- PLANS/global-codex1-installer/specs/reviewer_writeback_authority_enforcement/RECEIPTS/2026-04-17-reviewer-writeback-authority-proof.md

## Interfaces And Contracts Touched

- `capture-review-truth-snapshot` caller authority.
- `record-review-outcome` parent-owned writeback authority.
- Reviewer-lane bounded output contract.
- Qualification delegated-review proof.

## Implementation Shape

Add a machine-checkable authority boundary that distinguishes parent/orchestrator writeback from reviewer-lane execution. The implementation may use explicit caller context, a parent lease/capability binding, environment-based caller claims, or another deterministic mechanism, but it must satisfy these constraints:

- Reviewer lanes can still call `record-reviewer-output`.
- Reviewer lanes cannot mint a usable `review_truth_snapshot` writeback token.
- Reviewer lanes cannot call `record-review-outcome` successfully by setting `reviewer = "parent-review-loop"`.
- Parent review-loop can still capture truth, launch reviewers, collect reviewer-output refs, and record clean/blocked/replan outcomes.
- Mission-close review cannot terminalize from child-owned writeback.

## Proof-Of-Completion Expectations

- Runtime regression rejects reviewer-lane capture of parent writeback authority.
- Runtime regression rejects reviewer-lane `record-review-outcome` even with valid reviewer-output refs and a valid-looking snapshot.
- Runtime regression proves parent-owned writeback still works.
- Qualification CLI proof covers the authority boundary and remains green.
- `cargo test -p codex1 --test runtime_internal` passes.
- `cargo test -p codex1 --test qualification_cli` passes.
- `cargo fmt --all --check` passes.
- `cargo check -p codex1` passes.

## Non-Breakage Expectations

- Existing bounded reviewer-output artifacts remain readable.
- Contaminated review waves can still route to replan.
- Mission-close review remains mandatory before terminal completion.

## Review Lenses

- correctness
- review honesty
- authority boundary
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

- `contradiction:0d8f3968-2f41-460d-9ab8-3c08660c3aac`
- `.ralph/missions/global-codex1-installer/closeouts.ndjson:53`
- `PLANS/global-codex1-installer/REVIEW-LEDGER.md`

## Freshness Notes

- Based on mission-close contamination observed on 2026-04-17.
- Re-read current runtime writeback/capture implementation before editing because the review authority code has active recent changes.

## Support Files

- `REVIEW.md` records per-spec review context and dispositions.
- `NOTES.md` holds non-authoritative local notes or spike results.
- `RECEIPTS/` stores proof receipts that support completion claims.
