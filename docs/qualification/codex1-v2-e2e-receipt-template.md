# Codex1 V2 End-to-End Qualification Receipt (Template)

Copy this file to `codex1-v2-e2e-receipt.md` and fill in every `<TODO>`.
The qualification script (`scripts/qualify-codex1-v2.sh verify`) checks
for the three required marker lines below.

## Required markers

```text
skill_invocation: autopilot
ralph_hook: passed
verdict: complete
```

These MUST appear verbatim somewhere in the receipt — the verify step
greps for them.

## Mission context

- mission_id: `<TODO qual-<ts>>`
- repo_root: `<TODO absolute path of the tempdir>`
- runner: `<TODO codex | claude-code | other>`
- runner_version: `<TODO>`
- codex1 binary: `<TODO path/to/codex1>`
- codex1 version: `<TODO output of `codex1 --version`>`

## Session summary

Describe the live `$autopilot` session:

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
rather than a CLI-only simulation.

```text
<TODO paste transcript lines>
```

skill_invocation: autopilot

## Ralph hook evidence

Paste one or more log entries showing `ralph-status-hook.sh` ran at a
stop boundary and either allowed or blocked stop correctly.

```text
<TODO paste hook log lines>
```

ralph_hook: passed

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

verdict: complete

## Operator

- Name: `<TODO>`
- Signed: `<TODO RFC-3339 timestamp>`
- Notes: `<TODO any caveats or follow-ups>`
