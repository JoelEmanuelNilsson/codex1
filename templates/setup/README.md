# Setup Template Notes

Render the files in `templates/setup/project/` into the target repo root.

## Merge Intent

- `AGENTS.md`: if the repo already has an `AGENTS.md`, insert or update only
  the managed Codex1 block.
- `.codex/config.toml`: merge key-wise when the repo already has project Codex
  config.
- `.codex/hooks.json`: keep exactly one authoritative Ralph Stop pipeline.

## Placeholders

Replace these placeholders before installation:

- `{{BUILD_COMMAND}}`
- `{{TEST_COMMAND}}`
- `{{LINT_COMMAND}}`
- `{{ADDITIONAL_CHECK_1}}`
- `{{ADDITIONAL_CHECK_2}}`
- `{{REPO_SPECIFIC_NOTES}}`
- `{{STOP_HOOK_COMMAND}}`
- `{{SKILLS_BRIDGE_PATH}}` when using a non-default skill install mode

## Current Grounding

The config and hook shapes here follow the current official Codex docs for:

- project-scoped `.codex/config.toml`
- lifecycle hooks loaded from `.codex/hooks.json`
- one `Stop` hook acting as the authoritative continuation decision pipeline
