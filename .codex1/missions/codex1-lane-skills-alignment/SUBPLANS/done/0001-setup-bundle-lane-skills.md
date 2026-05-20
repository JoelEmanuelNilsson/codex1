## Slice Type

AFK. The PRD and setup contract define the required managed files and behavior.

## Execution Lane

standard

## Current Behavior

Codex1 setup installs the core workflow skills and supporting docs only. Lane skills such as TDD, diagnose, architecture improvement, and prototype still live outside the managed Codex1 setup bundle.

## Desired Behavior

Codex1 setup installs the core workflow skills plus full repo-local lane skills for `tdd`, `diagnose`, `improve-codebase-architecture`, and `prototype`.

## Key Interfaces

- `codex1 setup install`
- `codex1 setup status`
- `codex1 setup uninstall`
- Managed setup marker
- `SPECS/0001-codex1-lane-setup-contract.md`

## Scope

- Add managed bundle entries for the four lane skill `SKILL.md` files.
- Add expected file bodies for the lane skills.
- Bump the setup bundle version.
- Keep setup repo-local and non-destructive.
- Preserve existing setup safety behavior for symlinks and contained writes.
- Preserve uninstall semantics for managed files and mission artifacts.

## Out Of Scope

- Editing global skill directories.
- Creating a dogfood replacement skill.
- Adding unrelated external workflow integration.
- Changing native `/goal` behavior.
- Rewriting setup into a new architecture.

## Dependencies

- `SPECS/0001-codex1-lane-setup-contract.md`

## Blocked By

None.

## Acceptance Criteria

- [ ] Setup installs all four lane skills into `.agents/skills/`.
- [ ] Setup status reports the expanded managed skill set.
- [ ] The setup marker contains the expanded managed file set and new bundle version.
- [ ] Uninstall removes managed lane skills without deleting mission artifacts.
- [ ] The lane skill bodies do not require global skill paths to exist.
- [ ] Existing setup behavior for current, missing, stale, invalid, dry-run, backup, and restore remains intact.

## Expected Proof

- CLI tests that inspect installed files, setup status JSON, uninstall results, and marker JSON.
- `cargo test`.

## Exit Criteria

The setup bundle can materialize, report, and remove repo-local lane skills while leaving the repo working.
