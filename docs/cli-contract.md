# CLI Contract

Codex1 commands are deliberately small. The CLI only does work that benefits from deterministic code and path-safety checks.

Native Codex `/goal` owns continuation, pause/resume, usage accounting, and completion. Codex skills own PRDs, plans, specs, subplans, reviews, proofs, triage, and closeout.

## Global Flags

```sh
codex1 --json --repo-root <path> --mission <id> <command>
```

`--json` wraps command output in a stable envelope:

```json
{ "ok": true, "data": {} }
```

Errors use:

```json
{
  "ok": false,
  "error": {
    "code": "ARGUMENT_ERROR",
    "message": "..."
  }
}
```

Error codes are mechanical: `ARGUMENT_ERROR`, `MISSION_PATH_ERROR`, `IO_ERROR`, `SETUP_BACKUP_ERROR`, `SETUP_RESTORE_ERROR`, and `SETUP_BUNDLE_ERROR`.

## Commands

`init` creates the standard mission folder tree under `.codex1/missions/<mission-id>/`. It creates directories only and leaves mission content to the workflow skills.

`setup` is the short form of `setup install`: it materializes or repairs repo-scoped Codex1 skills and guidance files in the current repository. `setup install` and `setup enable` are the explicit forms. The managed skills are workflow skills, lane skills, review helper skills, and handoff guidance. `--dry-run` on the explicit install/enable forms reports planned writes, removals, backups, and materialized files without changing repo files.

`setup disable` and `setup uninstall` remove only Codex1-managed setup files and managed guidance blocks. They do not delete mission artifacts, user-authored guidance, user skills, or native goal state.

`setup status` reports mechanical repo bundle state: managed marker, managed skill state, supporting docs, managed guidance, bundle materialization, backup count, warnings, and anti-oracle language.

`setup doctor` diagnoses repo guidance mechanics only. `setup backups list` and `setup backups restore <id> --force` list and restore setup backups for repo-scoped setup targets.

## Path Safety

Mission IDs are limited to ASCII letters, digits, `-`, and `_`. Absolute paths, separators, NUL bytes, dot segments, hidden path tricks, and names containing `..` are rejected.

Mission initialization is contained inside the repository and rejects symlinked mission path components.

## Non-Goals

The CLI does not author mission artifacts, manage execution state, compute task readiness, judge proofs or reviews, decide close safety, or manage native goals.
