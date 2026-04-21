# Round 12 Repair Plan

Baseline: `9421cf8 round-11 repairs: harden plan/outcome contracts`

Repair scope: the accepted round-12 P1/P2 findings from `meta-review.md`, plus tests/docs directly required by those repairs. Merge-target findings from already-accepted round-10/round-11 families should be fixed where the same codepaths are hot, but they are not counted as standalone round-12 items.

## Accepted Findings To Repair

P1:

- F05: `plan check` lets a changed locked plan replace the live DAG without a replan.
- F14: side-effectful review/close mutations can publish artifacts before the event/state commit succeeds.

P2:

- F04: `outcome check` / `outcome ratify` still bless malformed OUTCOME field domains and non-string junk entries.
- F07: `plan check` still does not verify `PLAN.yaml mission_id` against the active mission.
- F15: `plan scaffold` records success and bumps revision before confirming `PLAN.yaml` is writable.
- F16: bare `status` incorrectly degrades an in-repo zero-candidate `PLANS/` tree into the Ralph fallback.
- F17: `verify-installed` breaks the documented `INSTALL_DIR=<path>` flow when the install dir is relative.
- F22: `status` and `close check` can announce terminal readiness even when `close complete` is guaranteed to fail on `CLOSEOUT.md`.

## Merge-Target Repairs To Fold In Opportunistically

These are already-open bug families that touch the same files and should be repaired while the relevant logic is being changed:

- round-11 F01 family: `task next` close readiness should use shared close-readiness truth.
- round-11 F07 family: historical task-id reuse must also reject truth surviving only in `state.reviews`.
- round-11 F09 family and round-10 F11 family: dirty-review readiness / post-repair verdict drift.
- round-11 F10 family: replan-triggered missions must not allow stale-plan execution.
- round-11 F14 family and round-10 F14 family: superseded/late review truth must stale-audit instead of blocking current work.
- round-11 F16 / F17 / F18 families: mission-close missing-file contract, terminal stale audit, and closeout history truth.
- round-11 F19 / F20 families: review and mission-close boundary identity still need hard fencing across restart/replan.

## Repair Groups

### 1. Plan Integrity And Replan Boundaries

Findings:
- F05
- F07
- merged round-11 F07

Intended behavior:
- Once a plan is locked, `plan check` may only be idempotent on the same hash or may relock after an explicit replan/unlock path.
- A hash-changing locked plan must not silently replace the authoritative DAG.
- `PLAN.yaml mission_id` must match the active mission.
- Historical task IDs must remain reserved across both `state.tasks` and `state.reviews`.

Files to edit:
- `crates/codex1/src/cli/plan/check.rs`
- possibly `crates/codex1/src/state/readiness.rs`
- possibly shared helpers for historical ID reservation

Tests:
- Locked plan edited in place without replan fails `plan check` and leaves state unchanged.
- `PLAN.yaml mission_id` mismatch fails `plan check`.
- Reusing a historical review task ID after replan fails `plan check`.

Risks:
- Preserve the existing same-hash idempotent path and the backfill path for old states missing `plan.task_ids`.
- Avoid turning valid relock-after-replan flows into false positives.

### 2. Artifact/State Transaction Safety

Findings:
- F14
- F15
- merged round-11 F17 / F18 families where close artifacts are touched

Intended behavior:
- Canonical review and close artifacts must not become visible unless the corresponding event/state mutation commits successfully.
- `plan scaffold` must not advance revision or append a scaffold event until `PLAN.yaml` can be published.
- Mission-close late-output and history fixes should use the same safer publication discipline.

Files to edit:
- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/core/paths.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/plan/scaffold.rs`
- `crates/codex1/src/cli/close/closeout.rs`

Tests:
- Failed `append_event` for `review record` does not leave canonical review artifacts behind.
- Failed `append_event` for `close record-review` does not leave canonical mission-close artifacts behind.
- Failed `append_event` for `close complete` does not leave canonical `CLOSEOUT.md` behind.
- Unwritable `PLAN.yaml` causes `plan scaffold` to fail without bumping revision or appending a scaffold event.

Risks:
- This is the highest-risk repair group because it touches shared mutation sequencing. Keep changes minimal and strongly covered by tests.
- If helper signatures change, avoid breaking already-correct atomic-write paths elsewhere.

### 3. Outcome Validation And Close Readiness Contracts

Findings:
- F04
- F22
- merged round-11 F01 / F16 / F17 / F18 / F20 families where the same close paths are touched

Intended behavior:
- OUTCOME validation must enforce the actual contract:
  - `status` enum is restricted,
  - required lists contain non-empty strings,
  - `definitions` keys and values are non-empty strings.
- Close readiness must be shared across `status`, `task next`, `close check`, and `close complete`, including `CLOSEOUT.md` writability and proof-aware close blockers.
- Mission-close surfaces should stop drifting on missing findings-file contract and closeout history truth while these files are being changed.

Files to edit:
- `crates/codex1/src/cli/outcome/validate.rs`
- `crates/codex1/src/cli/outcome/check.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/cli/close/check.rs`
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/status/project.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- `crates/codex1/src/cli/close/closeout.rs`
- `docs/cli-reference.md`

Tests:
- Invalid OUTCOME `status` values fail `outcome check` and `outcome ratify`.
- Required OUTCOME list fields containing only junk/non-string entries fail validation.
- Invalid `definitions` value types fail validation.
- Missing proof after passed mission-close review keeps `task next` from advertising `close`.
- Bad `CLOSEOUT.md` target makes `status.close_ready` and `close check.ready` false.
- Missing mission-close findings file returns the intended error contract.
- Dirty-then-clean mission-close history renders truthfully in `CLOSEOUT.md`.

Risks:
- Keep error messages actionable; don’t collapse these into generic parse failures.
- Avoid reintroducing the earlier `task next` mission-close/terminal regressions while tightening close readiness.

### 4. Status And Install Surface Corrections

Findings:
- F16
- F17

Intended behavior:
- Bare `status` should only degrade to the Ralph `foundation_only` fallback when there is truly no `PLANS/` tree in scope.
- If a `PLANS/` tree exists but has zero missions, `status` should surface `MISSION_NOT_FOUND`.
- `verify-installed` should support the documented relative `INSTALL_DIR=<path>` flow by canonicalizing the install path before the `/tmp` smoke.

Files to edit:
- `crates/codex1/src/cli/status/mod.rs`
- `Makefile`
- `docs/install-verification.md` if the final behavior/doc wording needs tightening
- tests around status/install surfaces

Tests:
- Bare `status` from a cwd with empty `PLANS/` returns `MISSION_NOT_FOUND`.
- `make install-local verify-installed INSTALL_DIR=.tmp-install-rel` succeeds.

Risks:
- Preserve the benign bare-shell fallback for Ralph when no mission workspace exists at all.
- Keep the install verification smoke running from `/tmp`; the goal is to fix the relative-path handling, not to weaken the test.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Because this pass touches CLI/install behavior, readiness/status behavior, and public docs/contracts, `make verify-contract` is required.
