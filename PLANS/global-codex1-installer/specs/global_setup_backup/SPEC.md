---
artifact: workstream-spec
mission_id: global-codex1-installer
spec_id: global_setup_backup
version: 1
spec_revision: 4
artifact_status: active
packetization_status: runnable
execution_status: complete
owner_mode: solo
blueprint_revision: 6
blueprint_fingerprint: sha256:6250fe0d1ab5eda605be0adda7974655b796c3b253cd9c190d6c2b78b2b0bda7
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

Implement global setup with user-scope backups.

## In Scope

- Make `codex1 setup` plan and apply user-scope Codex runtime files under `CODEX_HOME` or `~/.codex`.
- Install or normalize the managed Codex1 Stop hook in the user-level `hooks.json`.
- Enable user-level `features.codex_hooks = true` in `config.toml`.
- Install the managed global Codex1 skill surface, including `$close`, under the user-level Codex runtime location.
- Ensure `codex1 init` can run after global setup without rejecting the managed user-level Codex1 Stop hook or creating a duplicate project Stop hook.
- Report actual changed paths in human output when global setup mutates user files.
- Create reversible user-scope backup manifests before mutating existing user files.
- Add or update tests that prove user-scope setup behavior with temp HOME/CODEX_HOME fixtures.
- Preserve the locked setup/init command boundary and avoid project mutation.

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
- Backup and test surfaces relevant to this spec.
- CLI integration tests.

## Read Scope

- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/restore.rs
- crates/codex1/src/commands/uninstall.rs
- crates/codex1/src/support_surface.rs
- crates/codex1-core/src/backup.rs
- crates/codex1/tests/qualification_cli.rs

## Write Scope

- crates/codex1/src/commands/setup.rs
- crates/codex1/src/commands/restore.rs
- crates/codex1/src/commands/uninstall.rs
- crates/codex1/src/support_surface.rs
- crates/codex1/tests/qualification_cli.rs

## Interfaces And Contracts Touched

- Public command report contract relevant to this spec.
- User-scope backup manifest entries for global setup.
- JSON report assertions in tests.

## Implementation Shape

Implement `codex1 setup` as a global user-scope support-surface transaction. Resolve the target Codex home from `CODEX_HOME` or `HOME/.codex`. Plan writes for `config.toml`, `hooks.json`, and the managed global skill files, back up previous contents before mutation, record manifest entries with `scope = "user"` and `origin = "codex1 setup"`, then apply through the existing support-surface transaction helper. If no changes are needed, report an idempotent no-op. Use temp HOME/CODEX_HOME fixtures in tests and keep all project mutations behind explicit `codex1 init`.

Review repair note: global setup backups must be consumable by existing recovery commands. `restore` and `uninstall` must accept manifests whose root is the user Codex home when the entries are user-scoped, while preserving project manifest behavior. Idempotence must be semantic for TOML/JSON, not raw formatting only.

Replan repair note: global setup must be enough for machine-wide skill discovery, not just hook/config activation. The managed public skills must include `$close`, and project `init` must accept the global managed Codex1 Stop hook as the single authoritative Ralph pipeline instead of rejecting the environment produced by `setup`.

## Proof-Of-Completion Expectations

- Targeted CLI tests prove `codex1 setup` writes only user-scope `config.toml`, `hooks.json`, and global managed skill files under temp CODEX_HOME/HOME.
- Tests prove existing user files are backed up before mutation with manifest entries marked `scope = "user"`.
- Tests prove setup is idempotent when the global config, hook, and skill surface are already desired.
- Tests prove `codex1 setup` installs `$clarify` and `$close` into the global skill surface.
- Tests prove `codex1 init` succeeds after global setup and does not add a duplicate project Stop hook when the global managed Stop hook is present.
- Tests prove human setup output reports actual changed paths when global setup mutates user files.
- Tests prove existing global setup backups can be restored or uninstalled through the CLI.
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
