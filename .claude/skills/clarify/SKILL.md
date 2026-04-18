---
name: clarify
description: Codex1 V2 mission-intake. Use when the user invokes $clarify, starts a new mission, or gives a vague outcome that needs an OUTCOME-LOCK before planning.
---

# $clarify (Codex1 V2)

Destroy planning-critical ambiguity and ratify `OUTCOME-LOCK.md` before a plan
is authored.

## When to use

- The user invokes `$clarify`.
- A new mission id is needed.
- A mission exists but `OUTCOME-LOCK.md`'s `lock_status` is still `draft`.

## Steps

1. If no mission exists yet, create one:
   ```bash
   codex1 init --mission <safe-slug> --title "<human title>" --json
   ```
   Mission IDs must match `^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$`.

2. Interview the user for the three required sections (if they are still
   placeholder text): Destination, Constraints, Success Criteria. Write
   concrete statements directly into `PLANS/<id>/OUTCOME-LOCK.md`.

3. Flip `lock_status: draft` → `lock_status: ratified` in the frontmatter.
   Bump `updated_at` to now (RFC 3339).

4. Validate:
   ```bash
   codex1 validate --mission <id> --json
   ```
   Must exit `ok: true`.

## Stop boundaries

- `$clarify` does **not** mutate `PROGRAM-BLUEPRINT.md` — that is `$plan`'s job.
- `$clarify` does **not** set task status or start a parent loop.
- If the user wants to pause mid-interview, they can; `$clarify` holds no
  active parent loop.

## Example

```bash
codex1 init --mission checkout-refactor --title "Unify checkout APIs" --json
# ...interview user, edit OUTCOME-LOCK.md...
codex1 validate --mission checkout-refactor --json
```
