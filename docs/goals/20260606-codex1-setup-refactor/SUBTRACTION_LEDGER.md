# Setup Refactor Subtraction Ledger

## Removed Or Superseded Authority

| Old authority or manual surface | Removed or superseded by | Result |
| --- | --- | --- |
| `MANAGED_SKILL_FILES`, `MANAGED_SUPPORTING_DOC_FILES`, and `MANAGED_BUNDLE_FILES` as separate current inventories in `src/setup/catalog.rs` | One `CURRENT_BUNDLE_ENTRIES` table with path, role, and expected body | One current inventory source drives marker generation, status categories, install writes, and drift checks. |
| Full historical `LEGACY_BUNDLE_FILES_V*` arrays | Compact `LegacyReleaseSpec` records plus shared historical group builders | Legacy marker recognition remains exact by file order without hand-copying every historical path list. |
| `expected_body` match as a separate current-body map | `BundleEntry` body ownership for current files, with special cases only for marker/guidance/retired body proof | Expected current bodies live with the current entry they validate. |
| Current skill/doc arrays in `tests/common/mod.rs` | Status and dry-run plan helper functions | Integration tests consume the public setup interface instead of mirroring the bundle inventory. |
| Retired execution prompt body duplicated in `tests/common/mod.rs` | Internal setup/catalog tests using catalog body proof | Retired-file removal remains tested without a second expected-body copy. |
| Large inline legacy marker JSON fixtures in `tests/setup.rs` | `legacy_marker_body_for_test` inside catalog tests and a checked-in current marker fixture where legacy was not the behavior under test | Legacy compatibility tests moved to the compatibility module; integration tests no longer carry historical marker archaeology. |
| Separate skill/doc install preflight and write loops in `src/setup/mod.rs` | One loop over `owned_file_entries()` for preflight and one for writes | Install/enable behavior follows the entry interface and keeps role-specific reasons. |
| Hard-coded updater source/search roots as the only execution path | Default roots plus `CODEX1_SETUP_SOURCE_ROOT`, `CODEX1_SETUP_SEARCH_ROOT`, and `CODEX1_SETUP_BIN` overrides | Local fixture tests and agent dry-runs no longer need to scan or mutate the real fleet. |
| Updater shell behavior with no local fixture coverage | `tests/setup_updater.rs` | Syntax, tracked marker discovery, untracked marker skip, empty-root refusal, dirty-source reporting, and path canonicalization are covered locally. |

## Edit Points

Before this refactor, adding or retiring one managed Setup bundle file required touching or reasoning through many separate places:

- file content
- path constants
- skill/doc category arrays
- current bundle array
- expected-body match arm
- checked-in marker JSON
- integration test inventories
- large legacy marker fixtures or body fixtures when retiring
- docs/operator instructions

After this refactor, the normal add/retire budget is at most three source/artifact edit points excluding file content:

1. `src/setup/catalog.rs` for `BUNDLE_VERSION`, the `CURRENT_BUNDLE_ENTRIES` row, and any legacy/body proof needed for safe retirement.
2. `.codex1/setup-bundle.json`, refreshed as an installed marker artifact and checked by `checked_in_marker_matches_generated_marker`.
3. Documentation only when the category, safety rule, or operator workflow changes.

Tests derive current inventory from the catalog or public setup output. No permanent manifest, generator, helper table, or duplicate current test inventory was added.

## Drift Checks

- `setup::catalog::tests::checked_in_marker_matches_generated_marker` fails when `.codex1/setup-bundle.json` disagrees with `CURRENT_BUNDLE_ENTRIES`.
- `setup::catalog::tests::expected_bodies_match_checked_in_bundle_files` fails when an entry body disagrees with the checked-in managed file.
- `setup::catalog::tests::current_bundle_entries_have_no_duplicates` fails on duplicate current paths.
- `setup::catalog::tests::current_bundle_entries_are_fully_classified` fails if role classification stops covering the current bundle.
- `tests/setup_updater.rs` keeps local fleet-prep behavior fixture-drivable without applying to the real fleet.

## Net Assessment

The refactor is net subtractive in authority and workflow:

- Current bundle authority went from several arrays plus test mirrors to one entry table and generated/checked artifacts.
- Legacy compatibility went from full path-array archaeology to compact release specs.
- Integration tests stopped owning bundle inventories.
- Updater validation no longer requires reasoning about the real `/Users/joel` scan path.

The codebase did gain focused contract tests and small updater injection points, but they replace old manual reasoning and duplicate inventories rather than adding a new source of truth.
