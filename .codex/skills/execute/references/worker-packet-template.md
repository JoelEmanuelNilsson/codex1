# Worker Packet Template

Build the worker spawn prompt by substituting fields from `codex1 --json task packet T<ID>` into the template below. Keep the angle-bracket placeholders literal until you substitute. Trim `spec_excerpt` to roughly 2000 characters if the packet returns more.

## Packet fields

From `data` of `codex1 task packet <ID>`:

- `task_id` — e.g. `T3`
- `title` — short task title
- `spec_excerpt` — first ~2000 chars from `specs/T<id>/SPEC.md`
- `write_paths` — allowed write globs (array of strings)
- `proof_commands` — proof commands the worker must run (array of strings)
- `mission_summary` — short mission summary drawn from `OUTCOME.md`

## Prompt template

```
You are a Codex1 worker for task <TASK_ID>.

Mission: <MISSION_SUMMARY>

Spec: <SPEC_EXCERPT>

Allowed write paths:
  <WRITE_PATHS as bullet list>

Proof commands:
  <PROOF_COMMANDS as bullet list>

You may:
  - edit files inside the allowed write paths,
  - inspect any file in the repo,
  - run the listed proof commands.

You must not:
  - modify OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl,
    any file under reviews/, or CLOSEOUT.md,
  - record review results or findings,
  - replan the mission,
  - mark the mission complete.

When done, report:
  - changed files (one-line summary per file),
  - each proof command run and its output,
  - any blockers or assumptions you hit.
```

## Substitution example

Given a packet like:

```json
{
  "task_id": "T3",
  "title": "Add outcome check subcommand",
  "spec_excerpt": "Implement `codex1 outcome check --json` ...",
  "write_paths": ["crates/codex1/src/cli/outcome/**"],
  "proof_commands": ["cargo test -p codex1 outcome"],
  "mission_summary": "Ship Codex1 CLI v3."
}
```

The rendered worker prompt starts:

```
You are a Codex1 worker for task T3.

Mission: Ship Codex1 CLI v3.

Spec: Implement `codex1 outcome check --json` ...

Allowed write paths:
  - crates/codex1/src/cli/outcome/**

Proof commands:
  - cargo test -p codex1 outcome
...
```

## Notes

- Do not paste the mission's full `OUTCOME.md` into the worker — use the `mission_summary` field. If the summary is missing, keep the mission excerpt to one short paragraph.
- Do not add extra tools or permissions to the worker prompt. If a worker needs access beyond `write_paths` / `proof_commands`, that is a signal to replan, not to widen the worker's scope inline.
- The template is role-generic. The skill caller selects the model (see `SKILL.md` Worker Model Defaults); this template does not encode model choice.
