# Round 13 Repair Plan

Baseline: `f9e1a1a round-12 repairs: harden plan and close truth`

Repair scope: the accepted round-13 P1/P2 findings from `meta-review.md`, plus tests/docs directly required by those repairs. Merge-target findings from already-accepted round-10/round-11/round-12 families should be fixed where the same codepaths are hot, but are not counted as standalone round-13 items.

## Accepted Findings To Repair

P1:

- F01: `outcome ratify` can re-ratify a changed `OUTCOME.md` after plan lock and silently change active worker/reviewer instructions without a replan.
- F02: `plan check` can downgrade a recorded hard-planning mission to `light` and bypass the hard-evidence lock gate.

P2:

- F03: `plan check` accepts unknown `review_profiles`, so invalid review tasks lock and later emit unusable profiles to `$review-loop`.
- F04: `close complete` is not actually idempotent per the published contract.
- F05: `review record` docs still advertise `REPLAN_REQUIRED`, but the threshold path succeeds with `replan_triggered: true`.
- F07: proof receipts can escape `PLANS/<mission>` through a symlinked mission-local `PROOF.md`.
- F18: `replan record` mutates terminal missions and leaves publicly contradictory terminal state.
- F21: the published custom-`INSTALL_DIR` verification recipe can fail or verify the wrong binary from `/tmp`.

## Merge-Target Repairs To Fold In Opportunistically

These are already-open bug families that touch the same files and should be repaired while the code is hot:

- round-12 F14 family: artifact publication before commit (`close complete`).
- round-12 F15 family: scaffold event-before-publish ordering.
- round-12 F16 family: `status` docs for empty `PLANS/` behavior.
- round-12 F22 family: close-readiness docs and remaining path-aware close blockers.
- round-11 F09 / round-10 F11 family: dirty-review readiness and repaired-target reopen drift.
- round-11 F10 family: replan-triggered missions still leak executable work through read surfaces.
- round-11 F14 family / round-10 F14 family: superseded and stale review truth after replan.
- round-11 F19 / F20 family: planned-review and mission-close boundary identity.
- round-10 F16 family: review dependency readiness for non-target dependencies.
- round-8 F18 docs family: `replan record` scalar-vs-array docs drift.

## Repair Groups

### 1. Lock-Time Mission Truth Freeze

Findings:
- F01
- F02
- F03

Intended behavior:
- Once the plan is locked, `OUTCOME.md` ratification cannot silently change mission-root truth during execute/review/close phases.
- `plan check` must not allow `effective` planning level to fall below `requested`, and must not contradict the previously recorded `plan choose-level` state.
- `review_profiles` in `PLAN.yaml` must be validated against the canonical allowed set before plan lock.

Files to edit:
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/cli/plan/choose_level.rs` if shared helpers are useful
- `crates/codex1/src/cli/review/packet.rs` only if packet assumptions need tightening
- docs describing outcome/plan/review-profile contracts

Tests:
- `outcome ratify` fails once `plan.locked=true` / execute-phase is active and leaves state unchanged.
- `plan check` rejects `planning_level.effective < planning_level.requested`.
- `plan check` rejects a `PLAN.yaml` planning level that contradicts the recorded choose-level state.
- `plan check` rejects unknown `review_profiles`.

Risks:
- Preserve legitimate pre-lock editing flows: clarified outcome updates before lock should still be possible if the repo intends them.
- Keep the choose-level/scaffold/check sequence coherent for older missions whose state may not have every field populated.

### 2. Proof And Terminal Safety

Findings:
- F07
- F18

Intended behavior:
- Mission-relative proof receipts must stay inside `PLANS/<mission>` and must not escape through symlinks.
- Terminal missions must reject `replan record` and remain fully immutable.
- While touching these surfaces, fold in the stale mission-close boundary family if the code naturally allows it.

Files to edit:
- `crates/codex1/src/cli/task/finish.rs`
- `crates/codex1/src/cli/close/check.rs`
- `crates/codex1/src/cli/review/packet.rs`
- `crates/codex1/src/core/paths.rs` for shared proof-resolution helper(s)
- `crates/codex1/src/cli/replan/record.rs`
- possibly `crates/codex1/src/state/readiness.rs` if terminal guards need shared helper logic

Tests:
- Relative proof symlink outside the mission is rejected by `task finish`.
- `close check` does not trust a symlinked mission-local proof path.
- `review packet` does not emit escaped mission-local proof paths.
- `replan record` on a terminal mission returns `TERMINAL_ALREADY_COMPLETE` and leaves state/events unchanged.

Risks:
- Preserve support for truly absolute proof paths if the repo intends that escape hatch; only mission-relative proof paths need containment.
- Terminal guard must be enforced in both dry-run and locked mutation paths.

### 3. Public Contract And Install Docs Cleanup

Findings:
- F04
- F05
- F21

Intended behavior:
- Public docs must describe the actual `close complete` recovery/idempotency semantics.
- `review record` docs must describe the threshold path as success with `replan_triggered: true`, not `REPLAN_REQUIRED`.
- Custom `INSTALL_DIR` docs must describe verification using the chosen install location, not a hard-coded default path.
- While touching docs, fold in the stale `status`/`close_ready` and `replan record` shape drift that finding review merged into earlier families.

Files to edit:
- `docs/cli-reference.md`
- `docs/cli-contract-schemas.md`
- `docs/install-verification.md`
- possibly `README.md` if the user-facing verification section duplicates the stale guidance

Tests:
- Prefer targeted integration coverage where possible:
  - existing terminal-closeout recovery test should remain green,
  - existing dirty-threshold review test should remain green.
- No new docs-specific harness is required unless a tiny grep-style assertion already exists naturally.

Risks:
- Keep docs narrowly aligned to actual behavior; don’t invent new semantics just to make the docs prettier.
- If docs are changed to match behavior, ensure they still align with the handoff intent rather than merely reflecting a bug.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Because this pass touches CLI behavior, proof/terminal safety, and public install/contract docs, `make verify-contract` is required.
