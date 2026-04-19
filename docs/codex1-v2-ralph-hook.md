# Codex1 V2 Ralph Status Hook

Ralph is the tiny stop-guard that prevents Codex from casually declaring
a mission done while the CLI says otherwise. V2's Ralph consumes **one**
command and nothing else:

```bash
codex1 status --mission <id> --json
```

The hook script lives at `scripts/ralph-status-hook.sh`. It has two
modes:

- **Scan mode (default, invoked by `.codex/hooks.json`):** no arguments.
  Scans `<repo-root>/PLANS/*/STATE.json` (default `<repo-root> = $PWD`),
  runs `codex1 status --mission <id>` on each mission, and blocks stop
  if any mission reports `stop_policy.allow_stop: false`. This is how
  Codex Stop hooks and Claude Code Stop hooks should invoke it — no
  mission-id plumbing required.
- **Single-mission mode (explicit):** `ralph-status-hook.sh <mission-id>
  [--repo-root <path>]`. Used by tests and by operators debugging a
  specific mission.

## Install

1. Make sure a V2 `codex1` is discoverable via
   `scripts/resolve-codex1-bin` (the Ralph hook delegates to the same
   resolver). Setting `CODEX1_BIN` or building `target/release/codex1`
   in this repo both work.
2. Register the hook at whichever Stop surface your runner exposes:
   - **Codex:** `.codex/hooks.json` (shipped in this repo) already
     points at scan mode. No further config required.
   - **Claude Code:** `.claude/settings.json` — recipe below.
3. Start your runner from the directory that contains `PLANS/` so the
   hook's default `$PWD`-based repo-root resolves to the right mission
   root.

Example `.claude/settings.json` entry (not committed — user-personal):

```json
{
  "hooks": {
    "Stop": [
      {
        "command": "${CODEX1_REPO_ROOT:-/Users/joel/codex1}/scripts/ralph-status-hook.sh"
      }
    ]
  }
}
```

## Exit codes

| Code | Meaning                                           |
|------|---------------------------------------------------|
| 0    | Stop allowed. Ralph is silent.                    |
| 1    | Stop blocked; per-mission reasons on stderr.      |
| 2    | Hook itself failed (missing binary, malformed JSON, etc.); treat as blocking. |

## Contract

Ralph never:

- inspects `PLANS/<id>/` files directly;
- invokes other `codex1` subcommands;
- makes routing decisions (those are the parent loop's job);
- spawns subagents.

Ralph only:

1. Lists mission ids under `<repo-root>/PLANS/*/` (scan mode) or takes a
   single mission id on the command line (explicit mode).
2. Runs `codex1 status --mission <id> --json` for each one.
3. Reads `stop_policy.allow_stop` (boolean) and `stop_policy.reason`.
4. Blocks stop when ANY mission reports `allow_stop: false`; else allows
   it.
5. Surfaces `next_action.display_message` per blocking mission so the
   parent knows what the CLI wants next.

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
