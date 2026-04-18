# Global Setup Failed-Review Repair Proof

Date: 2026-04-17
Mission: global-codex1-installer
Spec: global_setup_backup
Failed review bundle: 26194473-3804-46cd-b8af-af07f48d85ba

## Findings repaired

- P1: Relative `CODEX_HOME` could make global setup manifests target duplicated restore/uninstall paths.
- P2: Proof overclaimed global uninstall and human changed-path output coverage.

## Repair summary

- Global setup now normalizes `CODEX_HOME` through the same absolute-root path handling used by restore/uninstall before planning user-scope writes.
- Added a focused non-JSON global setup test proving human output lists actual changed paths.
- Added relative `CODEX_HOME` restore coverage proving real managed files are restored and no duplicated nested `codex-home` target appears.
- Added relative `CODEX_HOME` uninstall coverage proving real managed files are removed and no duplicated nested `codex-home` target appears.

## Verification

- `cargo fmt --all` completed.
- `cargo test -p codex1 --test qualification_cli setup_ -- --nocapture` passed with 18 passed, 0 failed.

## Evidence refs

- `crates/codex1/src/commands/setup.rs`
- `crates/codex1/tests/qualification_cli.rs`
- `reviewer-output:26194473-3804-46cd-b8af-af07f48d85ba:3bf47195-4225-452e-83b9-f4f02ae03f5d`
- `reviewer-output:26194473-3804-46cd-b8af-af07f48d85ba:57ab1927-59e2-4f8c-bc47-76bd42f707a5`

## Follow-up proof tightening

- Tightened `setup_human_output_lists_global_changed_paths` so it isolates the final `scope: global` human report before asserting changed-path entries. This prevents the proof from passing on preflight output alone.
- Reran `cargo test -p codex1 --test qualification_cli setup_ -- --nocapture`: 18 passed, 0 failed.
