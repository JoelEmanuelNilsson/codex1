# Round 7 Repair Plan

Date: 2026-04-21

This plan follows `docs/audits/round-7/meta-review.md`.

## Accepted Repair Set

1. Stale-writer precedence in task/review mutators, including proof/findings preflight cases.
2. Stale-writer precedence in `plan check`.
3. Live-DAG / superseded-task consistency across `plan waves`, `status`, and `task next`.
4. Mission-close clean dry-run should preview `consecutive_dirty: 0`.
5. CLI docs should use a real `replan record --reason` code.
6. Bare `status` should distinguish true no-mission from ambiguous multi-mission discovery.
7. `make verify-contract` should run the documented installed-binary `/tmp` smoke.
8. Path containment tests should cover absolute paths and symlink escapes.
9. Status/task-next tests should cover `unknown_side_effects` blockers.
10. Close-review staging failure should be tested with unchanged-state assertions.
11. Cleanup: clarify skill metadata summary should not omit `status` / `definitions`.

## Implementation Order

1. Fix stale-writer ordering.
   - Move `state::check_expected_revision` immediately after state load in `task start`, `task finish`, `review start`, and `review record`.
   - Load/check state before parsing/validating `PLAN.yaml` in `plan check`.
   - Add tests for stale plan validation, stale missing proof, stale missing findings, and unlocked-plan stale task/review gates.

2. Fix live-DAG/supersession consistency.
   - Choose one explicit live-DAG rule and apply it across `plan waves`, `status`, and `task next`.
   - Prefer a shared helper or matching filtering rules so reviews only surface with live awaiting-review targets.
   - Add tests for superseded review targets and superseded roots with downstream tasks.

3. Fix close-review dry-run and staging tests.
   - Make clean dry-run return predicted `consecutive_dirty: 0`.
   - Add a staging-failure test proving dirty mission-close review does not mutate state when review artifact write fails.

4. Fix status/install/docs workflow issues.
   - Add a distinct ambiguous-mission error path or structured context so bare `status` only falls back for true no-mission cases.
   - Extend `verify-installed` to run installed `init` and `status` from `/tmp`.
   - Change `six_consecutive_dirty` examples to `six_dirty`.

5. Add coverage hardening.
   - Test absolute mission ids, absolute specs, and symlink spec escape.
   - Test `unknown_side_effects` blockers in both status and task-next.
   - Clean up clarify skill metadata summary.

## Verification

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```
