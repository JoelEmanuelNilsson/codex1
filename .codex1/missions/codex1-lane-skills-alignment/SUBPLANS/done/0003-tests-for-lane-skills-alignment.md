## Slice Type

AFK. The expected behavior is observable through the CLI and installed files.

## Execution Lane

tdd

## Current Behavior

Existing tests cover the smaller setup bundle and current managed skill/doc counts. They do not prove lane skill installation, lane field guidance, or proof/QA docs.

## Desired Behavior

Tests fail before the implementation and pass after it. They prove the expanded bundle through public CLI behavior and installed artifacts.

## Key Interfaces

- `cargo test`
- `codex1 setup install`
- `codex1 setup status`
- `codex1 setup uninstall`
- Managed marker JSON
- Installed `SKILL.md` and docs text

## Scope

- Update expected managed skill/supporting doc lists.
- Add or adjust tests for installed lane skills.
- Add status tests for missing or stale lane skills.
- Add marker tests for expanded bundle version and file set.
- Add content assertions for execution lane guidance and proof/QA docs.
- Keep tests focused on observable behavior.

## Out Of Scope

- Testing private helper functions only.
- Browser UI tests.
- Native goal tests.
- Golden tests that make harmless wording changes painful.

## Dependencies

- `SUBPLANS/ready/0001-setup-bundle-lane-skills.md`
- `SUBPLANS/ready/0002-plan-docs-execution-lanes.md`

## Blocked By

None.

## Acceptance Criteria

- [ ] Tests prove all lane skills are installed.
- [ ] Tests prove status detects missing or stale lane skills.
- [ ] Tests prove uninstall handles managed lane skills explicitly.
- [ ] Tests prove subplan guidance includes the required `Execution Lane` and allowed lane values.
- [ ] Existing CLI tests still pass.

## Expected Proof

- `cargo test`.
- Failing-first evidence where practical, or a note explaining why the test was added alongside implementation.

## Exit Criteria

The repo has test coverage that would catch accidental removal of Codex1 lane skill support.
