---
artifact: workstream-spec
mission_id: review-lane-role-contract
spec_id: review_loop_skill_surface
version: 1
spec_revision: 1
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 10
blueprint_fingerprint: sha256:73643e5ae5a1c12e2c80c3b51aafda42fd133e8eb835b7c9d1e19d72be9bd665
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

Make `$review-loop` canonical and remove `$review` from managed skill/support surfaces.

## In Scope

- Add `.codex/skills/review-loop/SKILL.md` as the parent/orchestrator review-loop skill.
- Remove `.codex/skills/review/SKILL.md` as a public skill name.
- Update managed skill registry/discovery expectations, AGENTS scaffold text, runtime-backend docs, qualification docs, and tests that currently list `review`.
- Keep direct reviewer agents prompt-only rather than introducing another skill.

## Out Of Scope

- Full child-lane Ralph isolation implementation.
- Full review-loop wave orchestration.
- Reopening the locked naming decision.

## Dependencies

- Outcome Lock revision 1.
- Existing support-surface skill copying, validation, setup, doctor, and qualification behavior.

## Touched Surfaces

- `.codex/skills/review-loop/SKILL.md`
- `.codex/skills/review/SKILL.md` removal
- `crates/codex1/src/support_surface.rs`
- `crates/codex1/src/commands/setup.rs`
- `crates/codex1/src/commands/doctor.rs`
- `crates/codex1/src/commands/qualify.rs`
- `crates/codex1/src/commands/uninstall.rs`
- `crates/codex1/tests/qualification_cli.rs`
- `docs/runtime-backend.md`
- `docs/internal-command-taxonomy-proposal.md`

## Read Scope

- .codex/skills
- crates/codex1/src
- crates/codex1/tests
- docs
- PLANS/review-lane-role-contract

## Write Scope

- .codex/skills
- crates/codex1/src
- crates/codex1/tests
- docs
- PLANS/review-lane-role-contract/specs/review_loop_skill_surface

## Interfaces And Contracts Touched

- Public Codex1 skill names.
- Managed support-surface skill validation and AGENTS scaffold.
- Qualification self-hosting skill-surface gate.

## Implementation Shape

Create the new `$review-loop` skill from the existing parent review workflow while making parent-only orchestration explicit. Remove the old `$review` skill file and update every managed source/test/doc that treats `review` as a required public skill. Do not create a direct-review skill replacement.

## Proof-Of-Completion Expectations

- Skill discovery/source validation expects `review-loop` and not `review`.
- AGENTS/support scaffolds name `review-loop` in the public skills stance.
- Qualification and support-surface tests pass after the skill rename.
- Stale public `$review` instructions are removed except historical contexts intentionally retained.

## Non-Breakage Expectations

- `clarify`, `plan`, `execute`, `autopilot`, `internal-orchestration`, and `internal-replan` remain available.
- Setup/doctor/qualification still recognize a valid managed skill surface.
- Parent review writeback remains through deterministic backend commands.

## Review Lenses

- spec_conformance
- correctness
- interface_compatibility
- evidence_adequacy

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

- `OUTCOME-LOCK.md`
- `.codex/skills/review/SKILL.md`
- `crates/codex1/src/support_surface.rs`

## Freshness Notes

- Current for Outcome Lock revision 1 and the implemented `$review-loop`
  support-surface migration.

## Support Files

- `REVIEW.md`
- `NOTES.md`
- `RECEIPTS/`
