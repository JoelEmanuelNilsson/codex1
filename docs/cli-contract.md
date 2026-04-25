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

`interview <kind> --answers <file>` validates answers and writes an artifact. Singleton artifacts fail on collision unless `--overwrite` is passed. Collection artifacts allocate unique numbered filenames.

`subplan move --id <id> --to <state>` safely moves one subplan file between lifecycle folders. It does not enforce one active subplan.

`inspect` reports artifact inventory and mechanical warnings only.

`receipt append --message <text>` appends an optional JSONL receipt.

`loop start|pause|resume|stop|status` manages `.codex1/LOOP.json`.

`ralph stop-hook` reads Stop-hook JSON from stdin and fails open unless explicit loop state says to block.

`doctor` runs fast diagnostics for template registration, path validation basics, loop schema version, and the installed binary path.

## Path Safety

Mission IDs are limited to ASCII letters, digits, `-`, and `_`. Absolute paths, separators, NUL bytes, dot segments, hidden path tricks, and names containing `..` are rejected.

Artifact writes are contained inside the mission directory and check symlink-resolved parents before writing.

## Non-Goals

The CLI does not compute task readiness, review cleanliness, proof sufficiency, PRD satisfaction, close safety, replan priority, graph waves, or terminal completion.
