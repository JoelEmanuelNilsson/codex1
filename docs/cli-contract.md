# CLI Contract

Codex1 commands are mechanical. They validate paths, render built-in templates, write artifacts, move subplan files, report inventory, and manage explicit loop state.

## Global Flags

```sh
codex1 --json --repo-root <path> --mission <id> <command>
```

`--json` wraps command output in a stable envelope:

```json
{ "ok": true, "data": {} }
```

Successful mutating commands may include forensic warnings without becoming failures:

```json
{
  "ok": true,
  "data": {},
  "warnings": [
    {
      "code": "EVENT_LOG_APPEND_FAILED",
      "message": "..."
    }
  ]
}
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

Error codes are mechanical: `ARGUMENT_ERROR`, `MISSION_PATH_ERROR`, `ARTIFACT_VALIDATION_ERROR`, `IO_ERROR`, `TEMPLATE_ERROR`, `INTERVIEW_ERROR`, `LOOP_ERROR`, `SETUP_ARGUMENT_ERROR`, `SETUP_CONFIG_PARSE_ERROR`, `SETUP_CONFIG_WRITE_ERROR`, `SETUP_BACKUP_ERROR`, `SETUP_RESTORE_ERROR`, and `SETUP_BUNDLE_ERROR`.

## Commands

`init` creates the standard mission folder tree.

`template list` and `template show <kind>` expose built-in v1 templates. There are no project or user template overrides.

`interview <kind> --answers <file>` validates answers and writes an artifact. Singleton artifacts fail on collision unless `--overwrite` is passed. Collection artifacts allocate unique numbered filenames. JSON mode requires `--answers` so stdout remains a parseable envelope.

`subplan move --id <id> --to <state>` safely moves one subplan file between lifecycle folders. It does not enforce one active subplan.

`inspect` reports artifact inventory and mechanical warnings only.

`receipt append --message <text>` appends an optional JSONL receipt.

`loop start|pause|resume|stop|status` manages `.codex1/LOOP.json`.

`ralph stop-hook` reads Stop-hook JSON from stdin and fails open unless explicit loop state says to block.

`setup install|enable|disable|uninstall|migrate|status|doctor|backups` manages Codex1 bundle availability and activation. Setup commands are about hook/config/skill/guidance activation only; they do not inspect or judge mission artifacts.

`doctor` runs fast diagnostics for template registration, path validation basics, loop schema version, the installed-command JSON envelope, and a loop/Ralph smoke check.

## Setup Contract

Setup treats Codex1 as a bundle of the `codex1` CLI, the Ralph Stop-hook adapter, Codex1 skills, Codex1 guidance, and mission artifact conventions. Global setup makes the bundle available on the machine. It does not make Codex1 active in every repository unless the user explicitly chooses all-repos activation.

The setup command surface is:

```sh
codex1 setup install [--mode off|allowlist|denylist|all] [--scope global|project] [--repo <path>] [--dry-run]
codex1 setup enable [--repo <path>] [--dry-run]
codex1 setup disable [--repo <path>] [--dry-run]
codex1 setup uninstall [--scope global|project] [--repo <path>] [--dry-run]
codex1 setup migrate --to global|project [--repo <path>] [--dry-run]
codex1 setup status [--repo <path>]
codex1 setup doctor [--repo <path>]
codex1 setup backups list
codex1 setup backups restore <id> [--force] [--dry-run]
```

The default activation mode for global setup is `allowlist`: `setup install` enables only the target repo, normally the current repo. `all` activation is valid only when requested explicitly. `off` disables Codex1 everywhere. `denylist` enables all repos except disabled entries. Activation policy is Codex1-owned global config, while repo-scoped skills and guidance are materialized into enabled repositories so they do not leak into unrelated repos.

Mutating setup commands plan every write, removal, and bundle materialization. With `--dry-run`, they return the planned edits without changing files. Without `--dry-run`, they back up every existing config file before mutation and record enough metadata to restore a previously missing file back to absence. Backups are setup metadata, not mission receipts.

JSON setup output uses the normal envelope. Successful mutation responses report mechanical fields such as activation mode, target repo, files written or removed, backups created, hook state, and bundle state. Setup errors use setup-specific mechanical codes for argument, parse, write, backup, restore, and bundle materialization failures.

`setup status` explains effective activation: global config presence, activation mode, repo policy result, global hook state, project hook state, repo bundle state, duplicate-hook risk, backup availability, and project-trust caveats. `setup doctor` diagnoses setup health such as executable availability, hook parseability, activation policy parseability, bundle materialization, duplicate hooks, and backup manifest health.

Setup must fail open for Ralph. If a setup policy is absent, invalid, unreadable, or cannot resolve the repo, Ralph falls back to existing explicit loop behavior or allows stop according to the applicable fail-open rule for that slice. Disabled repos must not receive loop pressure from setup gating.

## Mission Event Log

Codex1 keeps a mission-local forensic event log at `.codex1/events.jsonl` inside each mission directory. It records automatic metadata for mutating command outcomes such as initialization, artifact writes, subplan moves, receipt appends, and loop changes.

The log is append-only, best-effort, and non-authoritative. If appending an event fails after the real mutation succeeds, the command still succeeds and reports a warning in JSON mode or stderr in human mode. If a mutating command fails after a safe mission layout was resolved, Codex1 may append a small failure event and still returns the original command error.

Read-only commands do not append events: `template list`, `template show`, `inspect`, `doctor`, `loop status`, and `ralph stop-hook` stay read-only.

Event records contain small mechanical metadata: schema version, timestamp, mission id, command name, event kind, result, optional duration, artifact kind, template version, overwrite flag, lifecycle folders, booleans for message or reason presence, error code, and mission-relative paths. They do not contain raw argv, absolute paths, answer payloads, artifact body text, loop messages, receipt messages, review finding text, stdout, stderr, sequence numbers, or semantic status fields.

`inspect` reports only the count of parseable event entries and shallow mechanical warnings for malformed event log lines. It does not summarize last activity or infer progress, readiness, review state, close state, or next action from events.

## Path Safety

Mission IDs are limited to ASCII letters, digits, `-`, and `_`. Absolute paths, separators, NUL bytes, dot segments, hidden path tricks, and names containing `..` are rejected.

Artifact writes are contained inside the mission directory and check symlink-resolved parents before writing. Existing mission root components must be real directories, not symlinks.

## Non-Goals

The CLI does not compute task readiness, review cleanliness, proof sufficiency, PRD satisfaction, close safety, replan priority, graph waves, or terminal completion.
