# Setup Refactor Notes

## 2026-06-07 Baseline

- Current worktree is authoritative. `git status --short --branch` reports only `docs/goals/20260606-codex1-setup-refactor/GOAL.md` and `PREP.md` modified at start of execution, even though the prep artifact describes a larger dirty setup/skill-removal baseline.
- Read goal prep/audit, repo guidance, domain docs, setup docs, setup implementation, tests, marker, and updater script.
- Architecture decision: deepen the Setup bundle module around one entry interface that owns current inventory, roles, marker body, expected body lookup, retired/legacy recognition, and test-facing inventory. Avoid adding a separate manifest/generator unless it deletes a previous authority.
- TDD next slice: make tests consume bundle facts from CLI/runtime output or focused contract checks, not duplicated `MANAGED_SKILLS` / `MANAGED_SUPPORTING_DOCS` arrays in `tests/common/mod.rs`.

## Measurements

- Before refactor, `src/setup/catalog.rs` carried separate skill, doc, bundle, and legacy path arrays plus expected body matching and fingerprints.
- After catalog refactor, `src/setup/catalog.rs` has one `CURRENT_BUNDLE_ENTRIES` table for current inventory, roles, and expected bodies. Legacy recognition is expressed as release specs plus shared group builders instead of full historical arrays.
- Before refactor, `tests/common/mod.rs` duplicated current skill/doc inventories. After cleanup it derives paths from setup status and dry-run plan output.
- `.codex1/setup-bundle.json` remains the installed marker artifact; `setup::catalog::tests::checked_in_marker_matches_generated_marker` is the drift check against the bundle interface.
- Updater script: hard-coded source/search roots and shell-only validation path.

## Command Log

- `git status --short --branch`: only GOAL/PREP modified at current execution start.
- Read docs and source with `sed`; searched setup duplication with `rg`.
- `cargo test setup_install_materializes_repo_scoped_guidance`: passed before test inventory cleanup.
- Failed command attempt: `cargo test setup_install_materializes_repo_scoped_guidance setup_install_dry_run_does_not_materialize_files ...` because Cargo accepts only one test name filter before `--`.
- `cargo test --test setup`: passed after replacing duplicate current skill/doc arrays in `tests/common/mod.rs` with status/plan-derived helpers.
- `cargo test setup::catalog -- --nocapture`: passed after introducing the single current `BundleEntry` table and legacy release specs.
- `cargo test --test setup`: passed after `src/setup/mod.rs` switched install/status loops to catalog entries.
- Failed command attempt: `cargo test setup::tests setup::catalog -- --nocapture` for the same multiple-filter Cargo limitation.
- `cargo test setup:: -- --nocapture`: passed after moving legacy upgrade/removal proofs into internal setup/catalog tests.
- `cargo test --test setup`: passed after deleting legacy marker blobs from integration tests and using the checked-in current marker as a backup-restore fixture.
- `cargo test --test setup_updater`: first run failed because the updater compared `/var/...` fixture repo paths with Git-reported `/private/var/...` top-level paths and skipped a valid tracked marker.
- `cargo test --test setup_updater`: passed after canonicalizing discovered updater repo roots and test expectations. Covers bash syntax, tracked-marker discovery, untracked marker skip, empty search-root refusal, and dirty-source reporting.
- Updated `docs/setup-bundle.md` with the new maintenance interface, add/retire/change playbook, and validation ladder.
- Updated `.agents/skills/update-codex1-setups/SKILL.md` with fixture env vars and updater test command.
- Added `SUBTRACTION_LEDGER.md` with removed/superseded authorities and before/after edit points.
- `cargo fmt -- --check`: passed.
- `cargo test`: passed, 65 total tests across unit/integration targets.
- `cargo run --quiet -- --json setup status`: passed; marker, skills, supporting docs, guidance current; no warnings; anti-oracle text present.
- `cargo run --quiet -- --json setup doctor`: passed; bundle marker, managed files, guidance, and backup manifest checks ok.
- `cargo run --quiet -- --json setup install --dry-run`: passed with empty writes/removes/backups/materialized.
- `bash -n .agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh`: passed.
- `cargo test --test setup_updater`: passed, 4 updater fixture tests.
- `git diff --check`: passed.

## Next Step

- Make the updater script fixture-drivable without touching the real local fleet, then document the maintenance playbook and subtraction ledger.
