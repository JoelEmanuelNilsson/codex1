# cli_command_split Execution Receipt

- Mission: `global-codex1-installer`
- Spec: `cli_command_split`
- Original package: `2a96c5b5-df5c-4bdd-85f6-8931e9e9b049`
- Replanned package: `911b7252-9f9b-4c38-88fa-ba58562f3520`
- Writer packet: `988312f7-bcbc-4a2c-adb0-e750a766347c`

## Result

- Added public `codex1 init` command wiring.
- Preserved the former project-scoped setup behavior through `codex1 init`.
- Changed `codex1 setup` to a non-mutating global boundary report for this slice.
- Updated qualification helper smokes that need project support-surface repair to call `codex1 init`.
- Added a boundary test proving `codex1 setup` does not create repo-local `.codex` or `AGENTS.md`.
- Repaired review findings by updating project-facing doctor remediation text to `codex1 init`.
- Repaired review findings by making `codex1 setup --repo-root` fail loudly.

## Contradiction And Replan

- Contradiction: `f83fee05-2bfa-46d8-b04d-d084dc1f539d`
- Evidence: full `qualification_cli` initially failed because helper smokes still invoked `codex1 setup` for project repair.
- Resolution: reopened the execution package scope to include `crates/codex1/src/commands/qualify.rs` and switched those helper calls to `init`.

## Verification

- `cargo fmt --check` passed.
- `cargo check -p codex1` passed.
- `cargo test -p codex1 --test qualification_cli setup_does_not_create_project_local_codex_or_agents_files` passed.
- `cargo test -p codex1 --test qualification_cli setup_json_emits_preflight_to_stderr_before_final_report` passed.
- `cargo test -p codex1 --test qualification_cli doctor_marks_clean_setup_without_qualification_evidence_as_unsupported` passed.
- `cargo test -p codex1 --test qualification_cli setup_rejects_project_repo_root_flag` passed.
- `cargo test -p codex1 --test qualification_cli` passed before review repair: 27 passed, 0 failed.
- `cargo test -p codex1 --test qualification_cli` passed after review repair: 28 passed, 0 failed.
