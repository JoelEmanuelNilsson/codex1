---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: doctor_restore_verification
version: 1
spec_revision: 5
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 7
blueprint_fingerprint: sha256:300c94b1fc95900d8872119491999b64f3947e4b1ea881ec237aa09d9d7c14e7
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

Update doctor and restore verification for global setup and project init.

## In Scope

- Make `codex1 doctor` honestly report global setup health and project init health after the `setup` / `init` command split.
- Verify restore/uninstall behavior for user-scope global setup manifests and project-scope init manifests where the distinction matters.
- Add or update tests that prove doctor, restore, and uninstall behavior after global setup and project init.
- Preserve the locked setup/init command boundary.

## Out Of Scope

- Publishing or packaging releases.
- Mutating real user home during tests.
- Reverting unrelated work.

## Dependencies

- Locked Outcome Lock revision 1.
- Program Blueprint revision 1.
- Previous serial workstream is complete when listed in the blueprint dependencies.

## Touched Surfaces

- Command behavior relevant to this spec.
- Backup, doctor, restore, or test surfaces relevant to this spec.
- CLI integration tests.

## Read Scope

- crates/codex1/src/commands/doctor.rs
- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/src/commands/restore.rs
- crates/codex1/src/commands/uninstall.rs
- crates/codex1/src/support_surface.rs
- crates/codex1-core/src/backup.rs
- crates/codex1/tests/qualification_cli.rs

## Write Scope

- crates/codex1/src/commands/doctor.rs
- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/src/commands/restore.rs
- crates/codex1/src/commands/uninstall.rs
- crates/codex1/src/support_surface.rs
- crates/codex1/tests/qualification_cli.rs

## Interfaces And Contracts Touched

- Public command report contract relevant to this spec.
- Backup or doctor support-surface contract when applicable.
- JSON report assertions in tests.

## Implementation Shape

Implement this slice after its dependency workstream is complete. Use temp HOME/CODEX_HOME fixtures, preserve user-owned files through backups, and keep all project mutations behind explicit codex1 init. Doctor should not pretend global setup alone makes a project initialized, and restore/uninstall should keep user-scope and project-scope manifest handling distinct and safe.

## Proof-Of-Completion Expectations

- Targeted CLI tests prove doctor distinguishes global setup readiness from project init readiness.
- Targeted CLI tests prove restore/uninstall consume the right user-scope or project-scope manifest without crossing setup/init boundaries.
- Qualification helper flows prove full support requires global setup plus project init and can force-repair duplicate project Stop authority back to the single global managed pipeline.
- Tests assert no real home or unrelated repo files are mutated.
- Relevant cargo test -p codex1 subset passes.

## Non-Breakage Expectations

- Existing internal commands continue to compile.
- Existing project setup safety remains available through codex1 init.
- Managed backups still validate.

## Review Lenses

- correctness
- protected-surface safety
- backup/restore integrity
- test adequacy

## Replan Boundary

| Trigger code | Reopen layer |
| --- | --- |
| write_scope_expansion | execution_package |
| interface_contract_change | blueprint |
| dependency_truth_change | blueprint |
| proof_obligation_change | execution_package |
| review_contract_change | execution_package |
| protected_surface_change | mission_lock |
| migration_rollout_change | blueprint |
| outcome_lock_change | mission_lock |

## Truth Basis Refs

- OUTCOME-LOCK.md
- PROGRAM-BLUEPRINT.md
- crates/codex1-core/src/backup.rs

## Freshness Notes

- Based on source read during planning on 2026-04-17.
- Re-read current diffs before implementation because this repo has unrelated uncommitted changes.

## Support Files

- REVIEW.md records per-spec review context and dispositions.
- NOTES.md holds non-authoritative local notes or spike results.
- RECEIPTS/ stores proof receipts that support completion claims.
