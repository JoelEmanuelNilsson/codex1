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

Error codes are mechanical: `ARGUMENT_ERROR`, `MISSION_PATH_ERROR`, `ARTIFACT_VALIDATION_ERROR`, `IO_ERROR`, `TEMPLATE_ERROR`, `INTERVIEW_ERROR`, and `LOOP_ERROR`.

## Commands

`init` creates the standard mission folder tree.

`template list` and `template show <kind>` expose built-in v1 templates. There are no project or user template overrides.

`interview <kind> --answers <file>` validates answers and writes an artifact. Singleton artifacts fail on collision unless `--overwrite` is passed. Collection artifacts allocate unique numbered filenames. JSON mode requires `--answers` so stdout remains a parseable envelope.

`subplan move --id <id> --to <state>` safely moves one subplan file between lifecycle folders. It does not enforce one active subplan.

`inspect` reports artifact inventory and mechanical warnings only.

`receipt append --message <text>` appends an optional JSONL receipt.

`loop start|pause|resume|stop|status` manages `.codex1/LOOP.json`.

`ralph stop-hook` reads Stop-hook JSON from stdin and fails open unless explicit loop state says to block.

`doctor` runs fast diagnostics for template registration, path validation basics, loop schema version, the installed-command JSON envelope, and a loop/Ralph smoke check.

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
