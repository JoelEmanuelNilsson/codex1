## Slice Type

AFK. Proof expectations and closeout boundaries are defined by the PRD and plan.

## Execution Lane

proof-qa

## Current Behavior

The mission has a PRD and plan artifacts but no implementation proof, review evidence, or closeout for lane skill alignment.

## Desired Behavior

The mission records command proof and a closeout that checks completed work against the PRD, out-of-scope items, and known risks.

## Key Interfaces

- `cargo fmt --check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `codex1 setup status`
- `PROOFS/`
- `CLOSEOUT.md`

## Scope

- Run formatting, tests, lints, and setup status checks.
- Record successful commands and any accepted risks in `PROOFS/`.
- Audit PRD stories and out-of-scope boundaries.
- Write `CLOSEOUT.md` only after proof exists.

## Out Of Scope

- Broad exploratory Browser dogfood.
- Native goal completion.
- Creating a PR unless separately requested.
- Hiding failed proof.

## Dependencies

- `SUBPLANS/ready/0001-setup-bundle-lane-skills.md`
- `SUBPLANS/ready/0002-plan-docs-execution-lanes.md`
- `SUBPLANS/ready/0003-tests-for-lane-skills-alignment.md`

## Blocked By

None.

## Acceptance Criteria

- [ ] Formatting passes or the failure is fixed.
- [ ] Tests pass or failures are triaged with evidence.
- [ ] Clippy passes or warnings are fixed.
- [ ] Setup status is checked after implementation.
- [ ] Proof records include commands, results, and any failures.
- [ ] Closeout explains completed, deferred, superseded, risky, and out-of-scope work.

## Expected Proof

- `PROOFS/` artifact with command outputs summarized.
- `CLOSEOUT.md`.

## Exit Criteria

The mission can be honestly handed back as complete, or non-completion is recorded with concrete blockers and evidence.
