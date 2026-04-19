# Codex1 V2 Operator Guide

How to install, drive, and debug Codex1 V2 as a user.

## What V2 is (and is not)

V2 is a skills-first harness backed by a deterministic CLI contract
kernel. The CLI (`codex1`) owns mission file shape, validation, wave
derivation, review cleanliness, and mission-close readiness. Skills in
`.claude/skills/` (Claude Code)
and `.codex/skills/` (Codex; mirrored at T44) compose those CLI
commands into the user-facing workflow. Ralph is a thin stop-guard
that reads exactly one command: `codex1 status --mission <id> --json`.

V2 explicitly does **not**:

- store waves (they are derived),
- allow parent self-review,
- treat `events.jsonl` as operational truth,
- auto-discover missions (every command needs `--mission <id>`),
- walk up from the cwd to find the repo root (pass `--repo-root <path>`).

## Install

### Binary

The binary is `codex1`. Build it from source:

```bash
cd /path/to/codex1
cargo build -p codex1 --release
cp target/release/codex1 ~/.local/bin/
```

**About V1 on PATH.** Some machines already have `~/.cargo/bin/codex1`
from the V1 support CLI. V2 does not try to install over it — instead,
every skill and script resolves the V2 binary explicitly via
`scripts/resolve-codex1-bin`, which probes candidates for the V2 help
surface (`mission-close` and `parent-loop` subcommands) and rejects any
binary that doesn't match. Resolution order:

1. `$CODEX1_BIN` (if set and V2)
2. `<repo-root>/target/release/codex1` (if V2)
3. `<repo-root>/target/debug/codex1` (if V2)
4. `codex1` on PATH (only if V2)
5. `codex1-v2` on PATH (only if V2; legacy-development fallback)

All V2 skills begin with:

```bash
CODEX1="$("${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/resolve-codex1-bin")" || {
  echo "V2 codex1 not found. Set CODEX1_REPO_ROOT=<codex1 checkout> or build with: cargo build -p codex1 --release" >&2
  exit 1
}
```

and then use `"$CODEX1" <subcommand>` for every invocation. This is the
single source of truth for "where is the V2 binary" — no skill or script
should invoke bare `codex1` and hope PATH is right.

`CODEX1_REPO_ROOT` is the one knob operators need to set when the
Codex1 checkout lives somewhere other than `/Users/joel/codex1` (the
author's local default, kept as a graceful fallback).

### Ralph hook

A V2-shaped `.codex/hooks.json` ships in the repo. It invokes
`scripts/ralph-status-hook.sh` on every Stop event with **no arguments**.
The hook then scans `<cwd>/PLANS/*/STATE.json` and asks `codex1 status`
about each mission it finds; if any mission reports
`stop_policy.allow_stop: false`, stop is blocked with a per-mission
reason list.

This means Ralph enforcement is first-class and automatic — there is no
hidden env-var plumbing between the skills and the hook. Activating a
parent loop via `codex1 parent-loop activate` writes to `STATE.json`;
the hook reads `STATE.json` on the next stop event and sees the loop.
No manual handoff, no opportunity to silently fail open by forgetting a
`CODEX1_MISSION` export.

Claude Code has a symmetrical slot at `.claude/settings.json` — V2 does
not write that file (it is user-personal), but the recipe is identical:
```json
{
  "hooks": {
    "Stop": [
      { "command": "${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/ralph-status-hook.sh" }
    ]
  }
}
```

**Why scan-mode.** V2 refuses ambient mission *resolution* for CLI
commands (every `codex1` subcommand takes `--mission <id>` explicitly).
But the Stop hook asking "does any mission in this repo want to block
right now?" is a well-defined repo-state query, not an ambiguous lookup.
The hook reads the same authoritative `STATE.json` the CLI wrote, so
there is no separate source of truth to drift.

See `docs/codex1-v2-ralph-hook.md` for the full contract table.

**Migrating from a cached V1 hook config.** V2 does not answer
`codex1 internal ...` subcommands. If your runner has a cached Codex
hook config that invokes `codex1 internal stop-hook`, refresh it to
point at `scripts/ralph-status-hook.sh` before the next stop boundary —
otherwise the cached command will fail with `unrecognized subcommand`
and stop will block. There is intentionally no compatibility shim: a
fail-open shim created a silent allow-stop surface inside the V2 binary
and was removed in the Round 3 honesty fixes.

## Running a V2 session

One environment variable is enough for the common case — where the V2
checkout lives:

```bash
export CODEX1_REPO_ROOT=/path/to/codex1
```

This tells every skill's resolver preamble where to find
`scripts/resolve-codex1-bin`. The Stop hook reuses the same variable to
locate `scripts/ralph-status-hook.sh`. No `CODEX1_MISSION` export is
needed — the hook discovers active missions from `PLANS/` on its own.

If you run Codex or Claude Code from a subdirectory, the hook's
default `PWD`-based repo-root still needs to resolve to the mission
root. Start your runner from the same directory that contains `PLANS/`,
or pass `--repo-root` when invoking the hook manually for debugging.

### Skills

V2 skills live at repo-local `.claude/skills/<name>/SKILL.md`. Claude
Code picks them up when run with `--project-dir <repo>`. If you want
them globally available, symlink or copy them to
`~/.claude/skills/<name>/SKILL.md` yourself — V2 does not touch
user-global skill paths.

## Day-to-day flow

```
$clarify ──► $plan ──► $execute ◀─► $review-loop ──► mission-close
   │                      │                           │
   └─── $close pauses any active loop ────────────────┘
```

Manual invocation:

```bash
codex1 init --mission demo --title "Demo mission"
# invoke $clarify → ratify OUTCOME-LOCK.md
# invoke $plan → author PROGRAM-BLUEPRINT.md tasks
# invoke $execute (repeats):
codex1 task next   --mission demo
codex1 task start  --mission demo T1
# ...write code + PROOF.md...
codex1 task finish --mission demo T1
# invoke $review-loop:
codex1 review open   --mission demo --task T1 --profiles code_bug_correctness
codex1 review submit --mission demo --bundle B1 --input reviewer.json
codex1 review close  --mission demo --bundle B1
# ...once all tasks review_clean:
codex1 review open-mission-close --mission demo --profiles mission_close
# ...submit + close...
codex1 mission-close check    --mission demo
codex1 mission-close complete --mission demo
```

Autopilot runs all of that end-to-end:

```bash
# from your Codex / Claude Code session
$autopilot --mission demo
```

## JSON contract essentials

Every command supports `--json`. The stable success shape:

```json
{ "ok": true, "schema": "codex1.<command>.v1", ... }
```

Error shape:

```json
{
  "ok": false,
  "schema": "codex1.error.v1",
  "code": "<CODE>",
  "message": "...",
  "retryable": <bool>,
  "exit_code": <int>,
  "hint": "...",
  "details": { ... }
}
```

The status envelope (`codex1.status.v1`) is what Ralph consumes. The
primary field is `verdict`; all other fields are internally consistent
with it. See `docs/codex1-v2-cli-contract.md` for the full schema.

## Error codes you will hit

| Code                       | Exit | Meaning |
| -------------------------- | ---- | ------- |
| `MISSION_ID_INVALID`       | 2    | --mission slug violates the safe-slug regex |
| `MISSION_EXISTS`           | 3    | `init` on an existing PLANS/<id>/ directory |
| `MISSION_NOT_FOUND`        | 2    | command on a missing mission |
| `LOCK_INVALID`             | 2    | OUTCOME-LOCK.md structure rejected |
| `BLUEPRINT_INVALID`        | 2    | PROGRAM-BLUEPRINT.md YAML parse/shape error |
| `DAG_*`                    | 2    | DAG validation failure (ID, cycle, dep, schema) |
| `STATE_CORRUPT`            | 2    | STATE.json structure or cross-check failure |
| `REPO_ROOT_INVALID`        | 2    | --repo-root missing or not a directory |
| `PROOF_INVALID`            | 2    | proof file missing / empty / escape attempt |
| `TASK_STATE_INVALID`       | 2    | illegal status transition (e.g. finish before start) |
| `STALE_OUTPUT`             | 2    | binding mismatch; output quarantined |
| `REVISION_CONFLICT`        | 4    | --expect-revision mismatch (retryable) |
| `IO_ERROR`                 | 5    | filesystem failure (retryable) |
| `INTERNAL_ERROR`           | 70   | bug — file a report |

## Troubleshooting

**`Ralph blocked stop: active_parent_loop`**: the parent loop is active and
not paused. Use `$close` (→ `codex1 parent-loop pause`) to allow the
current turn to stop without losing the loop, or `codex1 parent-loop
deactivate` to abandon it.

**`STALE_OUTPUT` on `review submit`**: the reviewer bound to a stale
`task_run_id`, `graph_revision`, or `evidence_snapshot_hash`. Inspect
the binding in the error `details`, re-read current truth via
`codex1 task status`, and resubmit a reviewer output bound to the
current values.

**`REVIEW_BUNDLE_OPEN` during mission-close check**: a task bundle is
still open. Close it (clean or failed) before attempting mission-close.

**Six failed reviews on the same task**: `codex1 replan check` will
flag `six_consecutive_non_clean_reviews`. Invoke `$plan` to author a
superseding task; do not attempt a seventh retry on the same task id.

**Status says `invalid_state`**: the stored phase contradicts the task
distribution. Run `codex1 validate` for structural checks; inspect
`details.required_user_decision` for the specific contradiction.

## Qualification

V2 is considered "done" only when `docs/qualification/codex1-v2-e2e-receipt.md`
exists with the three required markers (`skill_invocation: autopilot`,
`ralph_hook: passed`, `verdict: complete`) — see
`docs/codex1-v2-skills-inventory.md` and
`scripts/qualify-codex1-v2.sh`.
