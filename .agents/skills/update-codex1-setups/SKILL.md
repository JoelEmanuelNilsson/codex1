---
name: update-codex1-setups
description: Update every valid local Codex1 setup repo on Joel's machine using the current /Users/joel/codex1 checkout.
---

# Update Codex1 Setups

Use when the user asks to update, refresh, repair, or upgrade every local Codex1 setup repo.

Valid target: an exact Git repo root with a tracked `.codex1/setup-bundle.json`. Do not update repos that only have untracked old setup files.

After editing setup-installed guidance, patch its canonical source first, usually `src/setup.rs`; `setup install` may overwrite `.agents/skills/*`.

Run the dry run first:

```bash
bash /Users/joel/codex1/.agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh --dry-run
```

If the dry run succeeds, apply:

```bash
bash /Users/joel/codex1/.agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh --apply
```

The apply mode refuses to run if `/Users/joel/codex1` has uncommitted changes, then pulls with `--ff-only`, builds the local binary, and runs `setup install` for each valid repo.
