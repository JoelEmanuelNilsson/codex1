# Spec Notes

- Mission id: `contract-centered-architecture`
- Spec id: `support_surface_txn`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Centralized the support-surface manifest schema and core rollback/write helpers in `codex1-core::backup`.
- Repointed `setup`, `restore`, and `uninstall` at the shared core manifest/write/rollback helpers instead of keeping separate local schema copies.
- Added a shared support-surface transaction executor in `codex1-core::backup` so apply/rollback/manifest-commit sequencing is no longer duplicated across `setup`, `restore`, and `uninstall`.
- Kept command-local preflight and policy branching where needed, but the mutation transaction path now runs through one shared core engine.
- Added a `qualification_cli` proof test that executes the real helper qualification flows against the built `codex1` binary.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
