# global_setup_backup Execution Receipt

- Mission: `global-codex1-installer`
- Spec: `global_setup_backup`
- Execution package: `4928be7e-3f5e-43e2-834c-41e6f8c402f4`
- Writer packet: `962460cf-ca84-452b-8ed2-fbd991b56ed5`

## Result

- `codex1 setup` now targets the user-level Codex home from `CODEX_HOME` or `HOME/.codex`.
- Global setup enables `features.codex_hooks = true` in user-level `config.toml`.
- Global setup installs or normalizes the managed Codex1 Stop hook in user-level `hooks.json`.
- Global setup creates a reversible backup manifest before mutating existing user files.
- Manifest entries for global setup use `scope = "user"` and `origin = "codex1 setup"`.
- Global setup remains idempotent when the user-level surface is already desired.
- Global setup treats semantically desired TOML/JSON as idempotent even when formatting differs.
- Existing global setup backups can be restored through the CLI against user-scope manifests.
- Project-local `.codex` and `AGENTS.md` mutation remains behind `codex1 init`.

## Verification

- `cargo fmt --check` passed.
- `cargo check -p codex1` passed.
- `cargo test -p codex1 --test qualification_cli setup_writes_global_codex_home_with_user_scope_backups` passed.
- `cargo test -p codex1 --test qualification_cli setup_is_idempotent_for_global_codex_home` passed.
- `cargo test -p codex1 --test qualification_cli setup_is_semantically_idempotent_for_global_codex_home` passed.
- `cargo test -p codex1 --test qualification_cli restore_accepts_global_setup_user_scope_backup` passed.
- `cargo test -p codex1 --test qualification_cli` passed before review repair: 30 passed, 0 failed.
- `cargo test -p codex1 --test qualification_cli` passed after review repair: 32 passed, 0 failed.
