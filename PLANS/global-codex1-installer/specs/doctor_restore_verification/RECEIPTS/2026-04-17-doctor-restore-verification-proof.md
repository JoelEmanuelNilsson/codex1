# Doctor Restore Verification Proof

- Mission: `global-codex1-installer`
- Spec: `doctor_restore_verification`
- Fresh execution package: `28fc5626-b2af-408b-ac0b-ec1c49f8e6da`
- Spec revision: `5`

## Proof Rows

- `cargo test -p codex1 qualification_cli_support_surface_helper_flows_pass -- --nocapture`
  - Result: passed.
  - Proves helper qualification now models the split lifecycle: global `codex1 setup`, project `codex1 init`, force repair of duplicate project Stop authority, and clean support-surface recovery.
- `cargo test -p codex1 --test qualification_cli doctor_ -- --nocapture`
  - Result: 9 passed, 0 failed.
  - Proves doctor distinguishes global setup readiness from project init readiness and qualification metadata remains structured.
- `cargo test -p codex1 --test qualification_cli setup_ -- --nocapture`
  - Result: 19 passed, 0 failed.
  - Proves global setup backup behavior, setup/init boundary behavior, relative `CODEX_HOME` restore/uninstall, and related setup qualification paths.
- `cargo test -p codex1 --test qualification_cli`
  - Result: 37 passed, 0 failed.
  - Proves the full public CLI qualification integration suite remains green after the setup/init/doctor boundary changes.
- `cargo fmt --all --check`
  - Result: passed.
- `cargo check -p codex1`
  - Result: passed.

## Behavior Proved

- `codex1 setup --json` now exposes a structural `backup_id` for user-scope global setup manifests, so restore/uninstall proofs do not have to scrape human notes.
- Project `codex1 init` rejects project-level authoritative Stop hooks when the global managed Stop hook is already installed.
- Project `codex1 init --force` removes project Stop authority in that case, leaving the global managed Stop hook as the single authoritative pipeline.
- Qualification helper flows no longer treat project-only `init` as enough for full doctor support; full support is global setup plus project init.
