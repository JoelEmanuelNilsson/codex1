# Readiness Audit

## Summary

Status: unblocked for a local implementation goal that refactors Codex1 setup maintenance, preserves the current dirty setup changes, validates locally, and does not run real fleet apply/commit-push.

No required blocker remains for that scoped goal, so `GOAL.md` is valid. Actual fleet apply, commit, and push remain intentionally out of scope unless Joel explicitly asks for them in a later run.

## Capability Map

| Capability | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Source control for local refactor | safe during goal | `git status --short --branch` shows `main...origin/main` with existing setup/skill-removal changes and untracked `docs/diagrams/`, `docs/goals/`. | Future agent must preserve dirty baseline and avoid destructive git commands. |
| Source control for fleet apply | needs user decision | Updater script refuses apply modes when `/Users/joel/codex1` has uncommitted/untracked changes. | Not required for the future goal if proof uses local fixture/dry-run tests. |
| Commits and pushes | needs user decision | Current request forbids commit/push in prep; no branch/PR policy was requested for execution. | Future goal should not commit, stage unrelated files, or push unless explicitly asked. |
| Rust toolchain | proven | `cargo --version`: 1.94.1. `rustc --version`: 1.94.1. | Local build and tests are available. |
| Git | proven | `git --version`: 2.50.1. | Required for repo fixtures and updater behavior. |
| Python for updater script | proven | `python3 --version`: 3.14.5. | Current updater uses Python to read marker JSON. |
| Bash for updater script | proven | `bash --version`: GNU bash 3.2.57. `bash -n update-codex1-setups.sh` passed. | Syntax is valid in current environment. |
| Existing test suite | proven | `cargo test` passed: 58 tests. | Includes setup, init, catalog, guidance, and codex-review helper tests. |
| Formatting gate | proven | `cargo fmt -- --check` passed. | Future goal should keep it green. |
| Setup status/doctor | proven | `cargo run --quiet -- --json setup status` and `setup doctor` passed; status reports current marker, skills, supporting docs, guidance, and 31 backups. | Mechanical only, not semantic readiness or completion proof. |
| Setup dry-run | proven | `cargo run --quiet -- --json setup install --dry-run` passed with no planned writes/removes/backups/materialized files. | Safe baseline for current source repo. |
| Fleet updater execution | safe during goal only in fixtures | Script was inspected and syntax checked; real fleet update was not run. | Future goal may add fixture tests or a temp-root mode. Do not scan/apply real `/Users/joel` fleet unless explicitly approved. |
| External services and credentials | proven unnecessary | No network service, secret, deployment, or paid API is needed for local refactor and tests. | Network access is irrelevant to completion. |
| Cost/quota | proven unnecessary | No paid API calls or cloud jobs needed. | Cost risk is none for scoped local work. |
| Production/data risk | safe during goal | Setup mutates repo files only when commands are run against a repo. | Future validation should use temp repos for mutation tests and avoid real fleet apply. |
| Safety constraints | proven by existing tests, must preserve | Tests cover unmanaged overwrite refusal, modified managed removal refusal, unmanaged marker refusal, dry-run no writes, backups/restore, and Mission scaffold preservation. | Future goal must add/retain these tests while refactoring. |

## Dirty Worktree Impact

The source checkout is dirty before this prep's artifacts:

- Deleted retired setup skills: `.agents/skills/codex1`, `.agents/skills/prototype`, `.agents/skills/brutal-review`.
- Modified setup bundle marker, setup catalog, setup install/removal logic, tests, and setup/docs text.
- Untracked `docs/diagrams/` and pre-existing `docs/goals/`.

This is not a blocker for local refactor execution if the future agent treats it as baseline and preserves it. It is a blocker for the current updater's `--apply` and `--apply-commit-push` modes because the script exits when the source repo is dirty.

## Required Future Preflight

Before implementation, the future goal should:

1. Run `git status --short --branch` and record the dirty baseline in `docs/goals/20260606-codex1-setup-refactor/notes.md`.
2. Re-run the focused baseline checks if the checkout changed: `cargo fmt -- --check`, `cargo test`, `setup status`, `setup doctor`, and `setup install --dry-run`.
3. Add tests before structural changes where behavior could regress.
4. Avoid real fleet update commands unless Joel explicitly authorizes them.

## Commands Run During Prep

- `sed` reads for `AGENTS.md`, `CONTEXT.md`, `README.md`, `docs/agents/*.md`, `docs/setup-bundle.md`, `docs/cli-contract.md`, setup source files, setup tests, and updater files.
- `git status --short --branch`
- `git diff --stat`
- `git diff --name-status`
- `git diff -- ...` for setup/source/test/doc baseline
- `rg --files ...`
- `rg -n "MANAGED_|LEGACY_BUNDLE|expected_body|LegacyBodyFingerprint|BUNDLE_VERSION|setup-bundle" ...`
- `wc -l src/setup/catalog.rs src/setup/mod.rs src/setup/workspace.rs tests/setup.rs tests/common/mod.rs .agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh docs/setup-bundle.md docs/cli-contract.md`
- `cargo fmt -- --check`
- `bash -n .agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh`
- `cargo test`
- `cargo run --quiet -- --json setup status`
- `cargo run --quiet -- --json setup doctor`
- `cargo run --quiet -- --json setup install --dry-run`
- `cargo --version`
- `rustc --version`
- `git --version`
- `python3 --version`
- `bash --version | head -n 1`

## Assumptions

- The future goal is allowed to edit code, tests, docs, and the setup updater locally.
- The future goal is not allowed to commit, push, stage unrelated files, or run real fleet update apply modes unless Joel explicitly asks.
- Existing uncommitted setup/skill-removal changes are intentional and should be preserved.
- Fixture-based updater validation is acceptable evidence for this refactor; actual fleet mutation is a separate operator action.
