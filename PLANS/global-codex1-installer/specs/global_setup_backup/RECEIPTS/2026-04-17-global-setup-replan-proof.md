# Replanned Global Setup Proof

Date: 2026-04-17
Mission: global-codex1-installer
Spec: global_setup_backup
Contradiction: 5224cd13-81ff-4da8-97f2-d4bcd30dd04c
Execution package: a8953e94-436b-4408-8d13-6611da826969

## What changed

- Reopened `global_setup_backup` after the prior clean state failed the machine-wide setup contract.
- Added global managed skill installation under `CODEX_HOME/skills`, including `$close`.
- Allowed `codex1 init` to coexist with the global managed Codex1 Stop hook without adding a duplicate project Stop hook.
- Fixed human global setup output so changed paths are listed when setup mutates user files.
- Added focused regression coverage for global skills and setup-to-init compatibility.

## Verification

- `cargo fmt --all --check` passed.
- `cargo test -p codex1 --test qualification_cli setup_ -- --nocapture` passed with 15 passed, 0 failed.
- `codex1 internal compile-execution-package` passed for `spec:global_setup_backup` as package `a8953e94-436b-4408-8d13-6611da826969`.

## Notes

- The repair preserves the setup/init split.
- The repair does not mutate real user home in tests.
- `doctor_restore_verification` remains the next downstream frontier after this reopened setup slice is reviewed cleanly.
