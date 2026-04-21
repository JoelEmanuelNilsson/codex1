# Round 16 Repair Plan

Baseline: `9619b632e07c28be6f32e52e10ca4ff46d8b68c0`

Repair scope: accepted round-16 P1/P2 findings from `meta-review.md`, plus tightly coupled tests/docs. Merged findings are repaired through their accepted parent items.

## Accepted Findings To Repair

P1:

- F04: mission-close dirty findings can be immediately marked clean without repair.
- F06: repaired dirty reviews still split status, readiness, and `task next`.
- F07: `task finish` can mutate stale-plan or terminal work because it uses only the locked-plan guard.
- F08: close artifacts can publish before state/event commit succeeds.
- F09: mission-close review can be recorded while the locked plan is invalid.
- F10: restarted planned-review boundaries still accept stale findings from the prior round.
- F17: replan relock can orphan active tasks and still allow terminal close.
- F18: superseded dirty planned-review truth survives replan/relock and blocks the rebuilt DAG.
- F20: dirty planned-review state commits before the findings artifact is durable.

P2:

- F01: spaced `INSTALL_DIR` paths are rewritten, and `verify-installed` falsely passes.
- F02: `outcome ratify` can fail after rewriting `OUTCOME.md`, leaving state unratified.
- F03: `outcome check` accepts valid YAML status-key spellings that ratify cannot rewrite.
- F05: terminal loop guard bypasses stale-writer conflict reporting.
- F11: late planned-review output during unlocked replan is audited without stable stale category/targets.
- F15: docs promise externally recorded absolute proof paths remain absolute, but review packets relativize repo-local external proofs.
- F16: same-sequence event recovery can attach the wrong audit event to a new mutation.
- F19: hard-plan evidence can be locked without an evidence summary.
- F21: pure-YAML outcomes render blank closeout summaries.
- F23: old review outputs after replan relock return `PLAN_INVALID` instead of stale audit.
- F24: `invalid_state` status can still set `stop.allow: true`.

## Repair Groups

### 1. Transaction Safety

Findings:

- F02
- F08
- F16
- F20

Intended behavior:

- Failed mutating commands must not leave canonical artifacts or audit entries describing uncommitted state.
- Event recovery for an existing same-seq trailing event must verify event identity before treating it as an idempotent retry.
- Planned-review dirty findings and close artifacts must be durable or safely recoverable before current truth points at them.

Files to edit:

- `crates/codex1/src/state/events.rs`
- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- Relevant tests in `outcome.rs`, `review.rs`, `close.rs`, and a state/event-focused test location.

Tests:

- Malformed or mismatched trailing event leaves `OUTCOME.md` and `STATE.json` unchanged on failed ratify.
- Same-seq matching event is reusable only when kind/payload match; mismatched event fails without state mutation.
- Review dirty artifact write failure does not commit accepted-current dirty state.
- Closeout and mission-close findings artifact write failures do not publish uncommitted artifacts.

### 2. Readiness And Guard Consistency

Findings:

- F05
- F06
- F07
- F09
- F24

Intended behavior:

- `--expect-revision` conflicts win before terminal semantic errors on loop commands.
- `task finish` uses the same executable-plan guard as `task start`.
- Dirty review repair freshness is shared by readiness, status, and `task next`.
- `close record-review` enforces locked-plan snapshot drift.
- Active invalid state must not allow Ralph stop.

Files to edit:

- `crates/codex1/src/cli/loop_/mod.rs`
- `crates/codex1/src/cli/task/finish.rs`
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/status/next_action.rs`
- `crates/codex1/src/state/readiness.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- `crates/codex1/src/cli/status/project.rs`
- Relevant tests in `loop_.rs`, `task.rs`, `status.rs`, `close.rs`, and Ralph hook tests.

Tests:

- Terminal loop command with stale `--expect-revision` returns `REVISION_CONFLICT`.
- `task finish` rejects terminal, replan-triggered, and locked-plan drift states.
- After repair, status and `task next` both advance to re-review and verdict is not contradictory.
- `close record-review` rejects locked-plan drift.
- Active invalid-state status sets `stop.allow=false`, and Ralph blocks.

### 3. Review, Replan, And Mission-Close Boundaries

Findings:

- F04
- F10
- F11
- F17
- F18
- F23

Intended behavior:

- Mission-close dirty output requires a repair/replan/new-review boundary before clean can pass; duplicate dirty records for the same boundary are not new rounds.
- Planned review restarts must not accept old findings as current truth.
- Stale review outputs during unlocked replan and after relock emit stable stale audit events.
- Replan relock must not orphan old non-terminal, non-superseded tasks outside the new DAG.
- Superseded dirty review truth must not block the rebuilt DAG.

Files to edit:

- `crates/codex1/src/state/schema.rs` if boundary identity is needed.
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/review/classify.rs`
- `crates/codex1/src/cli/replan/record.rs`
- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/state/readiness.rs`
- `crates/codex1/src/cli/close/check.rs`
- `crates/codex1/src/cli/close/record_review.rs`
- Relevant tests in `review.rs`, `replan.rs`, `plan_check.rs`, `close.rs`, and status tests.

Tests:

- Replaying old planned-review findings after restart is stale/audit-only.
- Unlocked and post-relock stale review outputs append a stable category and do not mutate current truth.
- Replan relock rejects or blocks omitted old in-progress tasks unless superseded.
- Superseded dirty review no longer blocks replacement DAG.
- Dirty mission-close cannot immediately clean or count duplicate dirties as separate rounds.

### 4. Contract And Artifact Follow-Through

Findings:

- F01
- F03
- F15
- F19
- F21

Intended behavior:

- Custom install paths with spaces install and verify the exact requested path.
- Ratify accepts the same YAML status key spellings that check accepts, or check rejects unsupported spellings consistently.
- Review packet preserves truly mission-external absolute proof paths.
- Hard evidence entries need non-empty summaries.
- Closeout reads `interpreted_destination` from both fenced and pure-YAML outcomes.

Files to edit:

- `Makefile`
- `docs/install-verification.md` if wording changes are needed.
- `crates/codex1/src/cli/outcome/validate.rs`
- `crates/codex1/src/cli/outcome/ratify.rs`
- `crates/codex1/src/cli/review/packet.rs`
- `crates/codex1/src/cli/plan/check.rs`
- `crates/codex1/src/cli/close/closeout.rs`
- Relevant tests in `outcome.rs`, `review.rs`, `plan_check.rs`, `close.rs`, and install/contract coverage if available.

Tests:

- `make install-local verify-installed INSTALL_DIR="/tmp/path with spaces"` installs to that exact path.
- `outcome check` and `ratify` agree for quoted `status` keys and space-before-colon status keys.
- Review packet keeps mission-external absolute proof paths absolute.
- Hard evidence without summary is rejected.
- Pure-YAML outcome closeout includes interpreted destination.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```
