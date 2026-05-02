# CLI Contract

Codex1 commands are mechanical. They validate paths, render built-in templates, write artifacts, move subplan files, append receipts, report inventory, and record small forensic events. They do not own long-running continuation.

Native Codex `/goal` is the only continuation primitive. Codex1 does not implement goal persistence, goal status, token or time accounting, automatic continuation, or goal completion.

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

Error codes are mechanical: `ARGUMENT_ERROR`, `MISSION_PATH_ERROR`, `ARTIFACT_VALIDATION_ERROR`, `IO_ERROR`, `TEMPLATE_ERROR`, `INTERVIEW_ERROR`, `SETUP_BACKUP_ERROR`, `SETUP_RESTORE_ERROR`, and `SETUP_BUNDLE_ERROR`.

## Commands

`init` creates the standard mission folder tree.

`template list` and `template show <kind>` expose built-in v1 templates. There are no project or user template overrides.

`interview <kind> --answers <file>` validates answers and writes an artifact. Singleton artifacts fail on collision unless `--overwrite` is passed. Collection artifacts allocate unique numbered filenames. JSON mode requires `--answers` so stdout remains a parseable envelope.

`subplan move --id <id> --to <state>` safely moves one subplan file between lifecycle folders. It does not enforce one active subplan.

`receipt append --message <text>` appends an optional JSONL receipt.

`setup install` and `setup enable` materialize or repair repo-scoped Codex1 skill and guidance files. `--dry-run` reports planned writes, removals, backups, and materialized files without changing repo files.

`setup disable` and `setup uninstall` remove only Codex1-managed setup files and managed guidance blocks. They do not delete mission artifacts, user-authored guidance, user skills, native goal state, or legacy continuation files.

`setup status` reports mechanical repo bundle state: managed marker, managed skill, managed guidance, bundle materialization, backup count, warnings, and anti-oracle language. It does not report hook state, native goal state, readiness, review state, proof sufficiency, or close safety.

`setup doctor` diagnoses repo guidance mechanics only. `setup backups list` and `setup backups restore <id> --force` list and restore setup backups for repo-scoped setup targets.

`inspect` reports artifact inventory and mechanical warnings only.

`doctor` runs fast diagnostics for template registration, mission id validation, the installed-command JSON envelope, and the anti-oracle posture.

Removed continuation command surfaces are intentionally absent and fail through the normal argument parser path. There are no compatibility shims.

## Mission Event Log

Codex1 keeps a mission-local forensic event log at `.codex1/events.jsonl` inside each mission directory. It records automatic metadata for remaining mutating command outcomes: initialization, artifact writes, subplan moves, and receipt appends, plus safe-layout failures for those command families.

The log is append-only, best-effort, and non-authoritative. If appending an event fails after the real mutation succeeds, the command still succeeds and reports a warning in JSON mode or stderr in human mode. If a mutating command fails after a safe mission layout was resolved, Codex1 may append a small failure event and still returns the original command error.

Read-only commands do not append events: `template list`, `template show`, `inspect`, `doctor`, and setup status/doctor stay read-only.

Event records contain small mechanical metadata: schema version, timestamp, mission id, command name, event kind, result, optional duration, artifact kind, template version, overwrite flag, lifecycle folders, error code, and mission-relative paths. They do not contain raw argv, absolute paths, answer payloads, artifact body text, receipt messages, review finding text, stdout, stderr, sequence numbers, native goal state, or semantic status fields.

`inspect` reports only the count of parseable event entries and shallow mechanical warnings for malformed event log lines. It does not summarize last activity or infer progress, readiness, review state, close state, goal state, or next action from events.

## Path Safety

Mission IDs are limited to ASCII letters, digits, `-`, and `_`. Absolute paths, separators, NUL bytes, dot segments, hidden path tricks, and names containing `..` are rejected.

Artifact writes are contained inside the mission directory and check symlink-resolved parents before writing. Existing mission root components must be real directories, not symlinks.

## Non-Goals

The CLI does not compute task readiness, review cleanliness, proof sufficiency, PRD satisfaction, close safety, replan priority, graph waves, terminal completion, native goal status, or continuation prompts.
