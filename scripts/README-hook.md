# Ralph Stop Hook

`scripts/ralph-stop-hook.sh` is the portable Codex1 Stop hook. It is the only
piece of Ralph. Ralph is intentionally tiny: the hook reads
`codex1 status --json` and decides whether to block or allow a Stop event.

## What it does

On every Codex Stop event the harness invokes the hook. The hook:

1. Runs `codex1 status --json` in the current working directory.
2. Fails closed if `codex1 status` returns a handled mission-resolution/config error.
3. Otherwise inspects `data.stop.allow` in the returned envelope.
4. Exits `0` when Stop is allowed (loop inactive, paused, or mission already
   past mission-close-review-passed / terminal-complete).
5. Exits `2` when Stop must be blocked (loop active, unpaused, and status
   reports `stop.allow == false`, or status failed with an explicit handled
   error such as ambiguous mission discovery / bad selector). Codex treats
   exit code 2 as "block this Stop and surface stderr to the user".

The hook does **not** read plan files, subagent state, `.ralph/` directories,
or any other artifact. It calls only `codex1 status --json`. If `codex1` is
missing, the status output is empty, or the JSON cannot be parsed, the hook
conservatively exits `0` so a broken install never wedges the terminal.

## Requirements

- `codex1` on `PATH`, or set `CODEX1_BIN=/absolute/path/to/codex1`.
- `jq` (strongly recommended; the fallback grep parse is best-effort).
- `bash` 4+.

## Install

Copy or symlink the script somewhere stable, then reference its absolute path
from your Codex hooks file.

Global user hooks live at `~/.codex/hooks.json`. Per-repo hooks live at
`./.codex/hooks.json`. Either works; per-repo is preferred when the repo
ships its own `codex1` workflow.

Example hook wiring:

```json
{
  "Stop": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "/absolute/path/to/codex1/scripts/ralph-stop-hook.sh"
        }
      ]
    }
  ]
}
```

You can also ask `codex1` to print the one-liner:

```bash
codex1 hook snippet
```

`codex1 hook snippet` only prints the wiring JSON; it does not install
anything.

## Verify

In a mission with loop active and `stop.allow == false`:

```bash
bash scripts/ralph-stop-hook.sh; echo "exit=$?"   # expect exit=2
```

After `codex1 loop pause --mission demo` (or when the mission is idle):

```bash
bash scripts/ralph-stop-hook.sh; echo "exit=$?"   # expect exit=0
```

You can also drive the hook against a synthetic JSON by stubbing `codex1`:

```bash
tmpdir=$(mktemp -d)
cat > "$tmpdir/codex1" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' '{"ok":true,"data":{"stop":{"allow":false,"reason":"active_loop","message":"Run $close to pause."}}}'
EOF
chmod +x "$tmpdir/codex1"
PATH="$tmpdir:$PATH" bash scripts/ralph-stop-hook.sh; echo "exit=$?"   # expect exit=2
```

## Uninstall

Remove the `Stop` entry from `~/.codex/hooks.json` (or the repo-local
`.codex/hooks.json`). The script itself is stateless; deleting the file is
safe at any time.

## Design notes

- Ralph is status-only. It never inspects mission files. This keeps the
  behavior consistent with whatever `codex1 status` reports today.
- Ralph does not enforce caller identity. Only the active main thread should
  feel Stop pressure; subagents should be prompt-governed to finish and exit
  on their own. Any additional filtering must be expressed through
  `codex1 status`, not by adding logic to the hook.
- The hook drains stdin if Codex provides hook input on the pipe, so the
  launcher never stalls even though the current implementation does not use
  the hook payload.
