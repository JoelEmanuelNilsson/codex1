# Codex1 V2 Ralph Status Hook

Ralph is the tiny stop-guard that prevents Codex from casually declaring
a mission done while the CLI says otherwise. V2's Ralph consumes **one**
command and nothing else:

```bash
codex1 status --mission <id> --json
```

The hook script lives at `scripts/ralph-status-hook.sh`. It must be
invoked with the active mission id; the parent thread that activated the
loop is responsible for passing that id through.

## Install

1. Ensure `codex1` (post-cutover) or `codex1-v2` (pre-cutover) is on
   `PATH`, or point `CODEX1_BIN` at the absolute path.
2. Register the hook at whichever stop-event surface your Codex runner
   exposes. For the Claude Code harness this is `settings.json`'s
   `hooks.Stop` entry; for the `codex` CLI it is the `hooks.stop`
   section of `~/.codex/config.toml`.
3. Pass the mission id to the hook. If your runner doesn't have a native
   way to thread the id through, write it to a small file the hook reads
   (the active mission is the sole thing the runner must know).

Example (`.claude/settings.local.json`):

```json
{
  "hooks": {
    "Stop": [
      {
        "command": "scripts/ralph-status-hook.sh",
        "args": ["$CODEX1_MISSION", "--repo-root", "$CLAUDE_PROJECT_DIR"]
      }
    ]
  }
}
```

## Exit codes

| Code | Meaning                               |
|------|---------------------------------------|
| 0    | Stop allowed. Ralph is silent.        |
| 1    | Stop blocked; message printed to stderr. |
| 2    | Hook itself failed; treated as blocking by most runners. |

## Contract

Ralph never:

- inspects `PLANS/<id>/` files directly;
- invokes other `codex1` subcommands;
- makes routing decisions (those are the parent loop's job);
- spawns subagents.

Ralph only:

1. Runs `codex1 status --mission <id> --json`.
2. Reads `stop_policy.allow_stop` (boolean) and `stop_policy.reason`.
3. Blocks stop when `allow_stop: false`; else allows it.
4. Surfaces `next_action.display_message` on block so the parent knows
   what the CLI wants next.

Status-envelope fields Ralph trusts:

```json
{
  "stop_policy": { "allow_stop": false, "reason": "active_parent_loop" },
  "next_action": { "display_message": "Start task T2." }
}
```

Every `stop_policy` combination Wave 4 emits:

| `active` | `paused` | `verdict`         | `allow_stop` | `reason`              |
|----------|----------|-------------------|--------------|-----------------------|
| true     | false    | any               | false        | `active_parent_loop`  |
| true     | true     | any               | true         | `discussion_pause`    |
| false    | *        | `complete`        | true         | `complete`            |
| false    | *        | `invalid_state`   | true         | `invalid_state`       |
| false    | *        | other             | true         | `no_active_loop`      |
