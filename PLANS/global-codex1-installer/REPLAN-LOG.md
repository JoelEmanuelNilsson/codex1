# Replan Log

Mission id: `global-codex1-installer`

## Notes

This log records non-local replans that reopen route, package, or mission truth.
Preserve valid work aggressively: each row names what remains trustworthy and carries forward.
Every replan must also name the invalidated assumption, package, review, or proof claim so stale truth does not silently survive.

| Replan id | Reopened layer | Trigger | Cause ref | Preserved work | Invalidated work | Artifact updates |
| --- | --- | --- | --- | --- | --- | --- |
| `f83fee05-2bfa-46d8-b04d-d084dc1f539d` | execution package | write_scope_expansion | qualification smoke failures still invoked project setup through `codex1 setup` | setup/init split | package `2a96c5b5-df5c-4bdd-85f6-8931e9e9b049` | widened `cli_command_split` scope to include `qualify.rs` |
| `review:f5f1d621-c8a0-4eb9-bab6-0ebd1bb01659` | execution package | review_contract_change | project-facing doctor text and `setup --repo-root` behavior were stale | setup/init split | prior repaired cli slice | widened `cli_command_split` scope to include `doctor.rs` |
| `5224cd13-81ff-4da8-97f2-d4bcd30dd04c` | blueprint | proof_obligation_change | clean `global_setup_backup` did not prove global skill installation or setup-to-init compatibility | command split, user-scope backups, semantic idempotence, restore/uninstall repair | clean review assumption for package `7771db17-e3db-422b-88d3-ff343f8c68a9` | reopen `global_setup_backup` proof to include global skill surface, `$close`, setup-to-init coexistence, and honest changed-path reporting |
| `e7198761-0b56-4978-b587-2aeaa944a03e` | blueprint | review_contract_change | clean `doctor_restore_verification` review was recorded with only spec/intent reviewer output and no code/correctness reviewer output | installer command split, global setup backup, doctor/restore implementation and proof rows | clean review event `f35f749e-15eb-4cc7-adb9-cc6a35172f90` as sufficient mission truth | add `review_lane_completion_guard` before accepting review-clean state or mission close |
| `0d8f3968-2f41-460d-9ab8-3c08660c3aac` | blueprint | review_contract_change | findings-only mission-close reviewer lanes called parent-owned review writeback and terminalized the mission | all installer implementation work, package `2a7a74b2-d786-4e52-9fe5-0b17b065c484`, clean `review_lane_completion_guard` review | terminal closeout `53` and mission-close review event `abeb9992-490b-4e64-a124-68f1289aa715` as acceptable completion truth | add `reviewer_writeback_authority_enforcement` before mission-close can be accepted |

## 2026-04-17 - cli_command_split execution package scope

- Contradiction: `f83fee05-2bfa-46d8-b04d-d084dc1f539d`
- Reopen layer: execution package.
- Preserved truth: the selected architecture remains `setup` global and `init` project-scoped.
- Invalidated truth: package `2a96c5b5-df5c-4bdd-85f6-8931e9e9b049` was too narrow because it excluded `crates/codex1/src/commands/qualify.rs`.
- Reason: `cargo test -p codex1 --test qualification_cli` showed qualification helper smokes still invoked `codex1 setup` for project repair after setup became a global boundary.
- Repair direction: include `qualify.rs` in the `cli_command_split` scope and switch helper smoke invocations that need project setup to `codex1 init`.

## 2026-04-17 - cli_command_split review repair scope

- Review: `f5f1d621-c8a0-4eb9-bab6-0ebd1bb01659`
- Reopen layer: execution package / targeted repair.
- Preserved truth: command split remains `setup` global and `init` project-scoped.
- Invalidated truth: the prior repaired slice missed project-facing `doctor` remediation text and allowed `setup --repo-root` to succeed as a no-op.
- Repair direction: include `doctor.rs` in the `cli_command_split` scope, switch project repair remediation text to `codex1 init`, and reject `codex1 setup --repo-root`.

## 2026-04-17 - global_setup_backup machine-wide setup proof

- Contradiction: `5224cd13-81ff-4da8-97f2-d4bcd30dd04c`
- Reopen layer: blueprint.
- Preserved truth: `codex1 setup` remains global/user-level, `codex1 init` remains explicit project setup, user-scope config/hooks backups remain the recovery model, and restore/uninstall user-scope repair remains valid.
- Invalidated truth: the clean `global_setup_backup` review assumption that config/hooks-only global setup satisfied machine-wide setup.
- Reason: fresh proof showed global setup did not install the global public skill surface, and the global managed Stop hook made project `init` reject the environment created by `setup`.
- Repair direction: reopen `global_setup_backup` proof to cover global skill installation, `$close` skill availability, setup-to-init compatibility without duplicate project Stop hooks, and honest human changed-path reporting before returning to `doctor_restore_verification`.

## 2026-04-17 - review lane completion guard

- Contradiction: `e7198761-0b56-4978-b587-2aeaa944a03e`
- Reopen layer: blueprint.
- Preserved truth: the setup/init command split, global setup backups, global skill installation, doctor/restore verification implementation, and their passing test evidence remain useful implementation work.
- Invalidated truth: review event `f35f749e-15eb-4cc7-adb9-cc6a35172f90` cannot be accepted as a sufficient clean review for `doctor_restore_verification`.
- Reason: the review gate was recorded clean with one durable spec/intent reviewer-output and no durable code/correctness reviewer-output, even though this was a code-producing slice and several code-review lanes failed to persist bounded outputs.
- Repair direction: add a new `review_lane_completion_guard` workstream that makes required reviewer-output profile coverage a backend-enforced condition for clean review writeback. Missing required lane outputs must block or replan review; they must not be converted to clean review by parent judgment or by a single unrelated reviewer-output.

## 2026-04-17 - reviewer writeback authority enforcement

- Contradiction: `0d8f3968-2f41-460d-9ab8-3c08660c3aac`
- Reopen layer: blueprint.
- Preserved truth: the setup/init split, global setup backup behavior, doctor/restore verification, review lane completion guard implementation, and their passing tests remain useful implementation work.
- Invalidated truth: mission-close closeout `53` and review event `abeb9992-490b-4e64-a124-68f1289aa715` cannot be accepted as terminal mission completion.
- Reason: findings-only mission-close reviewer lanes minted or used parent-owned review writeback authority and terminalized the mission instead of returning bounded reviewer-output only.
- Repair direction: add a new `reviewer_writeback_authority_enforcement` workstream that makes parent-owned review writeback authority non-mintable and unusable by reviewer lanes. The fix must be enforced in runtime/CLI qualification, not merely prompt text.
