---
name: update-codex1-setups
description: Update every valid local Codex1 setup repo on Joel's machine using the current /Users/joel/codex1 checkout, optionally committing and pushing only setup-managed changes.
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

To apply, commit, and push setup-only changes:

```bash
bash /Users/joel/codex1/.agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh --apply-commit-push
```

Apply modes refuse to run if `/Users/joel/codex1` has uncommitted changes, print the dirty source paths, then pull with `--ff-only`, build the local binary, and run `setup install` for each valid repo. Commit-push mode stages only setup-managed paths, commits `Update Codex1 setup guidance`, and pushes only branches that already match upstream; it skips repos with staged changes, no upstream, detached HEAD, or pre-existing ahead/behind commits.

For local fixture validation or agent dry-runs, the script accepts environment overrides:

```bash
CODEX1_SETUP_SEARCH_ROOT=/tmp/codex1-fixtures \
CODEX1_SETUP_BIN=/Users/joel/codex1/target/debug/codex1 \
bash /Users/joel/codex1/.agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh --dry-run
```

- `CODEX1_SETUP_SOURCE_ROOT` overrides the source checkout used for dirty checks, pulls, and builds.
- `CODEX1_SETUP_SEARCH_ROOT` overrides the fleet search root.
- `CODEX1_SETUP_BIN` skips the build and uses an existing executable.

Validate updater behavior without touching the real fleet:

```bash
cargo test --test setup_updater
```
