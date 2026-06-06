# Codex1 Setup Refactor Prep

## Objective

Prepare a future native `/goal` to refactor Codex1 setup maintenance so the Setup bundle is direct, safe, and agent-drivable to change across projects.

Codex1 setup is not optional skill decoration. It is Joel's universal repo setup layer: curated skills, managed docs, setup guidance, marker JSON, backups, upgrade/removal behavior, and the local fleet update path. The refactor should deepen the setup maintenance module so one clear interface owns current bundle inventory, retired managed files, legacy compatibility, expected bodies, validation, docs, and update flow.

This prep did not implement the refactor, did not commit or push, and did not run fleet updates. The parent session launched one nested Codex prep worker; that worker did not launch any additional nested Codex workers.

## Boundaries

- Preserve the Anti-Oracle Rule: Codex1 CLI, setup status, marker files, docs, and Mission scaffold output must not become native goal or workflow-state truth.
- Preserve setup safety: no overwriting user-authored files, no deleting user skills or Mission scaffold artifacts, no weakening backups, no path-containment regressions, and no unsafe legacy removal.
- Preserve the dirty worktree. Existing setup/skill-removal changes are baseline context, not something this mission may discard.
- Do not require real fleet apply, commit, or push to complete the refactor. The updater path can be validated with local fixture repos, dry-run behavior, syntax checks, and documented operator steps.

## Current Baseline

Required repo guidance read:

- `AGENTS.md`
- `CONTEXT.md`
- `README.md`
- `docs/agents/codex1-workflow.md`
- `docs/agents/codex1-domain.md`
- `docs/agents/codex1-artifact-briefs.md`
- `docs/setup-bundle.md`
- `docs/cli-contract.md`

Relevant implementation inspected:

- `src/setup/catalog.rs`
- `src/setup/mod.rs`
- `src/setup/workspace.rs`
- `src/cli.rs`
- `tests/setup.rs`
- `tests/common/mod.rs`
- `.agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh`

Measured facts:

- `src/setup/catalog.rs` is 789 lines and contains the current bundle version, 36 current bundle paths, 8 managed skill paths, 27 supporting doc paths, 10 current or legacy marker file sets, an expected-body `include_str!` match, 20 legacy managed body fingerprints, and catalog-specific tests.
- `tests/common/mod.rs` duplicates the current managed skill and supporting doc inventories with 8 skill paths and 27 supporting doc paths.
- `tests/setup.rs` is 796 lines and hardcodes legacy marker JSON path lists and current version expectations in behavioral tests.
- The updater script is 175 lines, hardcodes `/Users/joel/codex1`, scans `/Users/joel` for tracked setup markers, builds the local binary, and refuses apply modes when the source repo has any uncommitted or untracked change.
- Current worktree baseline has 23 tracked modified/deleted setup-related files plus untracked `docs/diagrams/` and `docs/goals/`.
- The currently dirty setup bundle is mechanically current according to `cargo run --quiet -- --json setup status`.

Friction found:

- The current bundle source of truth is spread across multiple shallow modules and artifacts. Retiring one skill fans out through constants, arrays, marker JSON, expected-body matching, legacy release lists, body fingerprints, docs, and tests.
- `catalog.rs` exposes too many maintenance facts. Callers and tests need to know lists, roles, versions, and legacy details that should sit behind one Setup bundle interface.
- `src/setup/mod.rs` loops separately over skills and supporting docs for preflight/write behavior. That role split is useful for status output, but the install flow should not need two hand-maintained inventories.
- Legacy compatibility is represented as full historical arrays plus ad hoc body fingerprints. The safety goal is valid, but the maintenance surface is too broad and easy to miss.
- Tests mirror implementation details instead of proving a smaller bundle contract. They duplicate current lists and embed large historical markers inline.
- The updater is useful but too script-shaped for agentic maintenance: no JSON summary, no fixture tests, hard-coded source root and search root, and apply mode cannot proceed from a dirty source checkout.

## What Should Be Refactored

1. Deepen the Setup bundle catalog module.
   - Create one authoritative bundle manifest/interface that drives current inventory, file roles, marker generation, expected bodies, current/retired classification, and status categories.
   - Expected impact: highest. It removes most edit fan-out.
   - Main risk: accidentally weakening safety around user-authored files or legacy removal.

2. Make install/status/uninstall flows consume bundle entries, not separate lists.
   - Preserve role-specific status output while iterating over one entry model.
   - Expected impact: high. It improves locality without expanding the CLI contract.
   - Main risk: subtle dry-run, backup, or ordering regressions.

3. Replace legacy array archaeology with a small compatibility model.
   - Keep exact known managed marker validation, but store/generate release manifests and retired managed file fingerprints through one documented path.
   - Expected impact: high for retirement work.
   - Main risk: too-permissive marker acceptance. The future implementation must bias toward refusal.

4. Convert tests into contract tests and fixtures.
   - Tests should prove behavior: current marker generation, no drift, install/enable/status/doctor, disable/uninstall safety, dry-run no writes, backups, legacy upgrade/removal, and updater fixture behavior.
   - Expected impact: medium-high. It prevents test code from becoming another catalog.
   - Main risk: losing coverage by deriving tests from the same broken source without independent fixture checks.

5. Improve the local fleet updater path.
   - Keep setup-only staging and conservative apply/push behavior.
   - Add agent-readable status or dry-run output and fixture coverage, or move the logic behind a small deterministic helper if that gives a better interface than shell.
   - Expected impact: medium. It removes late surprises like dirty-source refusal.
   - Main risk: accidentally making real fleet mutation too easy.

6. Write a maintenance playbook.
   - Document adding, changing, retiring, validating, publishing, and fleet-preparing the Setup bundle.
   - Expected impact: medium. It makes future agent work direct and token-efficient.
   - Main risk: docs drifting unless backed by automated checks.

## Success Metrics And Targets

- Current bundle inventory has one authoritative source in code or generated code. There are zero hand-maintained duplicate current bundle path lists in tests or marker artifacts.
- Adding or retiring one managed Setup bundle file requires at most 3 edit points outside the file content itself. Expected allowed edit points: the manifest/source table, optional legacy fingerprint/retirement data, and docs/playbook or changelog.
- Drift checks fail if `.codex1/setup-bundle.json`, expected bodies, current inventory, or docs-visible bundle counts disagree.
- Safety tests still cover refusal to overwrite unmanaged files, refusal to remove modified marker-owned files, refusing unmanaged marker paths, path containment, dry-run no writes, backups/restores, and Mission scaffold preservation.
- Legacy tests cover at least v1, the most recent pre-refactor version, the current skill-removal version, and a retired managed file with a fingerprint/body proof.
- Updater validation covers dirty-source reporting, valid tracked marker discovery, setup-only staging allowlist, skipped commit-push cases, and no real push in tests.
- Final validation passes:
  - `cargo fmt -- --check`
  - `cargo test`
  - `cargo run --quiet -- --json setup status`
  - `cargo run --quiet -- --json setup doctor`
  - `cargo run --quiet -- --json setup install --dry-run`
  - updater syntax and fixture tests
- Documentation explains the maintenance flow without implying Codex1 owns Native goal state, readiness, review, proof, or completion.

## Ranked Strategy

1. Define the target Setup bundle interface and write characterization tests first.
   - Impact: highest.
   - Risk: low if tests capture current behavior before restructuring.
   - Validation: current tests plus new contract tests for marker generation, role categorization, expected bodies, and legacy recognition.

2. Collapse current bundle data into one manifest/table and derive marker, categories, and expected bodies.
   - Impact: highest.
   - Risk: medium because `include_str!` bodies and marker JSON must remain exact.
   - Validation: generated marker equals checked-in marker, install/status tests pass, and an explicit duplicate-list guard passes.

3. Move legacy releases and retired file recognition behind a compatibility module.
   - Impact: high.
   - Risk: medium-high because unsafe deletion would be serious.
   - Validation: fixture markers for known releases, modified legacy file refusal, retired managed file removal only when body or fingerprint matches.

4. Simplify setup orchestration over entries.
   - Impact: medium-high.
   - Risk: medium.
   - Validation: dry-run plan assertions, backup assertions, and install/enable/disable/uninstall/status/doctor tests.

5. Refactor updater behavior with fixtures or a deterministic helper.
   - Impact: medium.
   - Risk: medium if real apply/push code changes.
   - Validation: shell syntax, fixture repos under temp dirs, dirty source simulation, no network/push in tests.

6. Update docs/playbook and close with review.
   - Impact: medium.
   - Risk: low.
   - Validation: docs mention add/change/retire/validate/publish/fleet-prepare, and no docs contradict the Anti-Oracle Rule.

## Rejected Alternatives

- Only update docs. Rejected because the main cost is architectural fan-out in the code and tests.
- Only add helper scripts around the current catalog. Rejected because the source of truth would remain duplicated.
- Accept any marker where every path is currently known. Rejected because it weakens exact managed marker safety.
- Move all setup state into runtime scanning of `.agents/skills`. Rejected because it would blur managed vs user-authored files and weaken deterministic setup behavior.
- Make `codex1 setup status` decide readiness or completion. Rejected by the Anti-Oracle Rule.
- Make actual fleet apply/commit/push part of the proof. Rejected for this mission because it needs user timing and a clean source repo; local fixture proof is the right validation surface.

## Validation Loop

The future goal should use a red-green-refactor loop for behavioral changes:

1. Record current state in `docs/goals/20260606-codex1-setup-refactor/notes.md`.
2. Add or adjust the smallest contract test that captures the next safety or maintainability target.
3. Refactor the module/interface to pass that test while keeping existing tests green.
4. Run the narrowest relevant command first, then broaden to full `cargo test`.
5. Re-measure edit points, duplicate lists, and docs drift.
6. Repeat until all success metrics are met.
7. Run closeout checks and an advisory `codex-review` if useful for the final diff.

Anti-gaming constraints:

- Do not satisfy edit-point metrics by hiding path lists in opaque strings, deleting legacy coverage, removing safety tests, weakening marker validation, or making tests derive everything from the exact code under test.
- Do not mark updater proof complete by skipping dirty-source, commit-push skip, or setup-only staging cases.
- Do not reduce the managed bundle surface just to reduce counts.
- Do not treat setup status/doctor as semantic proof of mission readiness or completion.

## Relevant Skills, Plugins, And Docs For Execution

Use these in the future execution session:

- `improve-codebase-architecture`: primary lane for deepening the Setup bundle module and improving locality/leverage.
- `tdd`: for behavior-preserving refactors and safety tests.
- `codex-review`: advisory closeout review after tests and cleanup.
- `diagnose`: only if a hard regression or failing updater behavior appears.
- Repo docs: `CONTEXT.md`, `README.md`, `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, `docs/agents/codex1-artifact-briefs.md`, `docs/setup-bundle.md`, `docs/cli-contract.md`.
- No external plugin or service is required for the implementation.
