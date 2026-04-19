---
name: close
description: Codex1 V2 discussion-mode gate. Use when the user invokes $close, interrupts with a question, or needs to pause an active parent loop without ending it.
---

# $close (Codex1 V2)

Pause the active parent loop so the user can talk, redirect, or inspect
state without Ralph forcing forward motion. A paused loop is still the
parent's loop — resume continues, deactivate discards.

## When to use

- The user invokes `$close`.
- The user asks a clarifying question mid-execution or mid-review.
- The parent needs time to decide between repair and replan.

## Binary resolver

Every skill starts by resolving the V2 `codex1` binary to `$CODEX1`.

```bash
CODEX1="$(/Users/joel/codex1/scripts/resolve-codex1-bin)" || {
  echo "V2 codex1 not found; build with: cargo build -p codex1 --release" >&2
  exit 1
}
```

Use `"$CODEX1"` for every `codex1` invocation below.

## Steps

1. Pause the parent loop:
   ```bash
   "$CODEX1" parent-loop pause --mission <id> --json
   ```
   After this, `"$CODEX1" status` returns `stop_policy.reason:
   discussion_pause` and Ralph allows stop.

2. Talk to the user. The CLI makes no further changes until resumed.

3. Resume when ready:
   ```bash
   "$CODEX1" parent-loop resume --mission <id> --json
   ```
   Or abandon the loop entirely:
   ```bash
   "$CODEX1" parent-loop deactivate --mission <id> --json
   ```

## Stop boundaries

- `$close` does **not** mutate mission files beyond `parent_loop.paused`.
- `$close` does **not** touch `OUTCOME-LOCK.md`, blueprint, or task state.
- This is **not** `mission-close`. Terminal close is driven by
  `"$CODEX1" mission-close check` + `"$CODEX1" mission-close complete`
  (Wave 5), composed inside `$autopilot`.

## Status behaviour while paused

```json
{
  "parent_loop": { "active": true, "mode": "execute", "paused": true },
  "stop_policy": { "allow_stop": true, "reason": "discussion_pause" }
}
```

`active: true` keeps `$execute`/`$review-loop` aware the loop still exists;
`paused: true` lets Ralph back off.
