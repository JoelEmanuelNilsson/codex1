# Codex1 V2 Operator Guide

How to install, drive, and debug Codex1 V2 as a user.

## What V2 is (and is not)

V2 is a skills-first harness backed by a deterministic CLI contract
kernel. The CLI (`codex1` post-cutover, `codex1-v2` pre-cutover) owns
mission file shape, validation, wave derivation, review cleanliness,
and mission-close readiness. Skills in `.claude/skills/` (Claude Code)
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

During Wave 4 development the binary is `codex1-v2`. Build it from source:

```bash
cd /path/to/codex1
cargo build -p codex1-v2 --release
cp target/release/codex1-v2 ~/.local/bin/
```

After T44 cutover the binary is `codex1`. Existing scripts should read
`CODEX1_BIN` and fall back to either name.

### Ralph hook

Register `scripts/ralph-status-hook.sh` at your runner's stop surface.
See `docs/codex1-v2-ralph-hook.md` for the full install table.

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
