# Codex1 Lane Skills Alignment Plan

## Mission Link

Serves `PRD.md` in `.codex1/missions/codex1-lane-skills-alignment/`.

## Outcome Contract

Codex1 setup installs the core workflow skills plus the four lane skills as repo-local managed files. Plans and subplans can name execution lanes without depending on global skills. Docs explain the simple model: clarify, create PRD, plan, then native `/goal` executes the mission using lane guidance. The old broad dogfood workflow is not revived.

Out of scope: global skill installation/removal, issue tracker integration, native goal state integration, a mega skill, broad default Browser dogfood, and making TDD mandatory where it does not fit.

Proof that matters: setup CLI tests, status/marker tests, installed content assertions, subplan lane contract assertions, docs assertions, and normal Rust formatting/tests/lints.

## Implementation Shape

- Setup bundle: expand managed skill constants, expected bodies, marker version, status aggregation, install/update/uninstall, and legacy marker handling.
- Lane skill content: import the original `tdd`, `diagnose`, `improve-codebase-architecture`, and `prototype` skill bodies as closely as possible, with tiny Codex1-local wrappers.
- Plan contracts: update plan skill references and templates so ready subplans require `Execution Lane`.
- Docs: update workflow/setup docs to explain core skills, lane skills, proof/QA, and native `/goal` boundary.
- Tests: cover behavior through CLI output, files written into temp repos, status JSON, bundle marker contents, and text assertions.

## Research Posture

No external research is needed. The needed sources are local: the existing Codex1 setup implementation, tests, docs, and the original local skills.

## Decision Artifacts

- `ADRS/0001-repo-local-lane-skills.md`
- `SPECS/0001-codex1-lane-setup-contract.md`

## Execution Order

1. Build the setup bundle expansion and lane skill materialization.
2. Update plan/subplan templates and workflow docs for execution lanes.
3. Add behavioral tests for setup, status, marker, docs, and lane contract.
4. Run proof/QA and record the evidence.

## Parallelization Notes

The first two implementation slices touch related setup/docs files and should usually run serially. Test updates can start after the setup contract is visible. Proof/QA runs last.

## Ready Subplans

- `SUBPLANS/ready/0001-setup-bundle-lane-skills.md`
- `SUBPLANS/ready/0002-plan-docs-execution-lanes.md`
- `SUBPLANS/ready/0003-tests-for-lane-skills-alignment.md`
- `SUBPLANS/ready/0004-proof-qa-and-closeout.md`

## Proof Strategy

- Run `cargo fmt --check`.
- Run `cargo test`.
- Run `cargo clippy -- -D warnings`.
- Run `codex1 setup status --repo-root /Users/joel/codex1 --json` or the equivalent built binary command after install/update behavior is verified.
- Record proof artifacts in `PROOFS/`.
- Write `CLOSEOUT.md` only after checking PRD stories and out-of-scope boundaries against proof.

## Risks And Non-Goals

- Risk: copying original skills can accidentally weaken them. Keep changes tiny and obvious.
- Risk: bundle marker changes can break existing setup refresh behavior. Tests must cover current and stale bundles.
- Risk: docs can make Codex1 feel heavier than intended. Keep the user flow short and lanes optional.
- Non-goal: no global skill cleanup.
- Non-goal: no issue trackers.
- Non-goal: no replacement dogfood skill in this mission.

## Human Decisions

None. The PRD already resolves the needed product and workflow decisions.
