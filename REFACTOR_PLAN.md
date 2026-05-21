# Codex1 Refactor Plan: Minimal CLI Boundary

## Status

Codex1 intentionally keeps only the CLI behavior that deterministic code is better at than Codex:

- `setup`: repo-local skill/guidance materialization, status, backups, enable/disable, uninstall, and setup doctor.
- `init`: path-safe creation of the standard mission directory tree.

Skills carry the mission workflow. Native `/goal` carries persistent objectives.

## Decision

Keep the CLI small enough that it cannot become a workflow oracle. It should install guidance, scaffold directories, protect paths, preserve backups, and stop.

## Proof

The remaining tests should prove:

- `init` creates the expected mission directories and rejects unsafe mission IDs or symlinked mission path components.
- `setup` materializes, reports, disables, enables, uninstalls, backs up, restores, and diagnoses only repo-local managed guidance.
- Unknown commands fail through the normal argument parser.
- Help output advertises only `init` and `setup`.
- Docs describe the smaller boundary.
