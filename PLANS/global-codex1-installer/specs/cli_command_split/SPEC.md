---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: cli_command_split
version: 1
spec_revision: 4
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 4
blueprint_fingerprint: sha256:cf607369dc06a4f92454ca033aee0c3874e3e62defde3c53afca493883de6e06
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
---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: cli_command_split
version: 1
spec_revision: 3
artifact_status: active
packetization_status: runnable
execution_status: packaged
owner_mode: solo
blueprint_revision: 3
blueprint_fingerprint: sha256:6e8dc3c37bfb5b894ba2cba361a69d99e5c50baef465ff620794613023bbdbdc
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

Add explicit project init and reserve setup for global machine setup.

## In Scope

- Add `init` to the public CLI command enum, args, dispatch, and help surface.
- Move or wrap current project-scoped setup behavior behind `codex1 init` without changing its safety model in this slice.
- Change `codex1 setup` entrypoint so it no longer calls project-scoped setup logic.

## Out Of Scope

- Implement full global setup file planning beyond a minimal boundary stub if that belongs to the next spec.
- Change backup manifest schema unless required for command split compilation.
- Publish or package the binary.

## Dependencies

- Locked Outcome Lock revision 1.
- Current setup implementation in crates/codex1/src/commands/setup.rs.
- Current CLI wiring in crates/codex1/src/main.rs and crates/codex1/src/commands/mod.rs.

## Touched Surfaces

- Public CLI command names and help.
- Project setup command module boundary.
- Existing setup-related CLI tests.

## Read Scope

- crates/codex1/src/main.rs
- crates/codex1/src/commands/mod.rs
- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/init.rs
- crates/codex1/src/commands/doctor.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/qualification_cli.rs

## Write Scope

- crates/codex1/src/main.rs
- crates/codex1/src/commands/mod.rs
- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/init.rs
- crates/codex1/src/commands/doctor.rs
- crates/codex1/src/commands/qualify.rs
- crates/codex1/tests/qualification_cli.rs

## Interfaces And Contracts Touched

- Public CLI command contract: setup vs init.
- SetupArgs/InitArgs command argument model.
- JSON setup/init report names if exposed by tests.

## Implementation Shape

Create a new init command module from the current project-scoped setup behavior. Register TopLevelCommand::Init and dispatch it to commands::init::run. Make setup::run global-only in contract, even if its full file-write behavior is completed by the next spec. Update tests that expected project setup under setup so they call init for project behavior and add a boundary test that setup does not create repo-local .codex or AGENTS.md.

Execution replan note: qualification helper smokes invoke the public binary for project support-surface repair. Those internal calls must also switch from `setup` to `init` in this slice, otherwise the command split leaves qualification gates exercising the new global setup boundary where they need the preserved project setup behavior.

Review repair note: project-facing `doctor` remediation text is part of the command UX contract. It must direct project repair to `codex1 init`, and stale `codex1 setup --repo-root` invocations must fail loudly rather than reporting success as a no-op.

## Proof-Of-Completion Expectations

- Targeted CLI tests prove codex1 init performs the former project-scoped behavior in temp repos.
- Targeted CLI tests prove codex1 setup does not write project-local .codex or AGENTS.md.
- Qualification helper smoke tests that intentionally repair project support surfaces use codex1 init after the split.
- Doctor remediation for project support surfaces points at codex1 init, not codex1 setup.
- codex1 setup rejects project-scoped --repo-root usage.
- cargo test -p codex1 reaches at least the setup/init command tests without compile errors.

## Non-Breakage Expectations

- Existing internal commands continue to compile.
- Existing project setup backup/trust behavior remains available through codex1 init.
- No real user home or real project outside test fixtures is mutated.

## Review Lenses

- command UX clarity
- behavior preservation
- protected-surface safety
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
- crates/codex1/src/main.rs
- crates/codex1/src/commands/setup.rs

## Freshness Notes

- Based on source read during planning on 2026-04-17.
- Re-read current diffs before implementation because the repo has unrelated uncommitted changes.

## Support Files

- REVIEW.md records per-spec review context and dispositions.
- NOTES.md holds non-authoritative local notes or spike results.
- RECEIPTS/ stores proof receipts that support completion claims.
