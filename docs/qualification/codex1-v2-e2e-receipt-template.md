# Codex1 V2 End-to-End Qualification Receipt (Template)

**Do not verify this file directly.** Copy it to
`docs/qualification/codex1-v2-e2e-receipt.md`, remove the `(Template)` from
the H1 above, replace every `TODO-FILL-IN` value in the JSON block below with
real session data, then run:

```bash
scripts/qualify-codex1-v2.sh verify docs/qualification/codex1-v2-e2e-receipt.md
```

The verifier refuses any file whose name ends in `-template.md` or whose
first H1 contains the word "Template", so this file can never be mistaken
for a completed receipt.

## Required evidence (machine-parsed)

The verifier extracts this fenced JSON block and rejects the receipt if:

- any required field equals `TODO-FILL-IN` or starts with `<TODO`;
- `skill_invocation != "autopilot"`;
- `ralph_hook != "passed"`;
- `verdict != "complete"`;
- `terminality != "terminal"`;
- `session_transcript_excerpt` is shorter than 40 characters after trimming.

```json
{
  "schema": "codex1.qualification.receipt.v1",
  "skill_invocation": "TODO-FILL-IN",
  "ralph_hook": "TODO-FILL-IN",
  "verdict": "TODO-FILL-IN",
  "terminality": "TODO-FILL-IN",
  "mission_id": "TODO-FILL-IN",
  "repo_root": "TODO-FILL-IN",
  "operator": "TODO-FILL-IN",
  "runner": "TODO-FILL-IN",
  "runner_version": "TODO-FILL-IN",
  "codex1_bin": "TODO-FILL-IN",
  "codex1_version": "TODO-FILL-IN",
  "started_at": "TODO-FILL-IN",
  "completed_at": "TODO-FILL-IN",
  "duration_seconds": 0,
  "task_count": 0,
  "review_bundle_count": 0,
  "repairs": 0,
  "replans": 0,
  "final_state_revision": 0,
  "final_event_seq": 0,
  "session_transcript_excerpt": "TODO-FILL-IN"
}
```

Expected values on a valid run (must match exactly):

- `skill_invocation: "autopilot"`
- `ralph_hook: "passed"`
- `verdict: "complete"`
- `terminality: "terminal"`

## Mission context

- mission_id: `<TODO qual-<ts>>`
- repo_root: `<TODO absolute path of the tempdir>`
- runner: `<TODO codex | claude-code | other>`
- runner_version: `<TODO>`
- codex1 binary: `<TODO path/to/codex1>`
- codex1 version: `<TODO output of `codex1 --version`>`

## Session summary

Describe the live `$autopilot` session in prose. The JSON block above is
what the verifier checks; this section is for humans reading later.

- Started: `<TODO RFC-3339 timestamp>`
- Completed: `<TODO RFC-3339 timestamp>`
- Duration: `<TODO minutes:seconds>`
- Number of tasks in blueprint: `<TODO>`
- Number of review bundles opened: `<TODO>`
- Number of repairs: `<TODO>`
- Number of replans: `<TODO>`
- Final state_revision: `<TODO>`
- Final last event seq: `<TODO>`

## Skill invocation trace

Paste enough of the runner's transcript to show that `$autopilot`
actually invoked `$clarify` / `$plan` / `$execute` / `$review-loop`
rather than a CLI-only simulation. The `session_transcript_excerpt`
field in the JSON block should mirror (or summarize) what you paste
here.

```text
<TODO paste transcript lines>
```

## Ralph hook evidence

Paste one or more log entries showing `ralph-status-hook.sh` ran at a
stop boundary and either allowed or blocked stop correctly.

```text
<TODO paste hook log lines>
```

## Final status envelope

Copy the output of:

```bash
codex1 status --mission <mission_id> --repo-root <tempdir> --json
```

taken immediately after `mission-close complete`.

```json
<TODO paste envelope>
```

Expected: `"verdict": "complete"`, `"terminality": "terminal"`,
`"parent_loop.active": false`.

## Operator

- Name: `<TODO>`
- Signed: `<TODO RFC-3339 timestamp>`
- Notes: `<TODO any caveats or follow-ups>`
