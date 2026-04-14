# Hook Script Location

Store the repo-local Codex1 hook implementation here when setup installs a
project-owned stop hook script.

## Required Contract

- one authoritative Ralph `Stop` decision pipeline only
- the command referenced by `.codex/hooks.json` should resolve from the git root
- the script should read current mission truth and the latest valid closeout
  instead of inventing workflow semantics on its own
- for `Stop`, return JSON on stdout when exiting `0`, or exit `2` and write the
  continuation reason to stderr

## Placeholder Linkage

`templates/setup/project/.codex/hooks.json` uses `{{STOP_HOOK_COMMAND}}` so setup
can either point at a rendered script in this folder or at another stable
repo-local command path.
