# Round 10 Repair Plan

Baseline: `9169e98 round-9 repairs: harden lifecycle concurrency`

Repair scope: only the accepted round-10 P1/P2 findings from `meta-review.md`, plus tests/docs directly required by those repairs. Non-standalone checklist items F04-F07 may receive opportunistic test strengthening if adjacent files are already touched, but they are not standalone product repairs.

## Accepted Findings To Repair

P1:
- F10: `close complete` can publish stale or rejected `CLOSEOUT.md` before the terminal mutation wins.
- F11: dirty-review repair keeps routing to `repair` after the target was repaired and is ready for re-review.

P2:
- F01: `outcome ratify` can fail after ratifying state.
- F02: `task next` collapses ready tasks from multiple topological waves into one `run_wave`.
- F03: `status.stop.allow` depends on close blockers despite the stop contract.
- F08: README and CLI reference give failing `task finish --proof` paths.
- F09: CLI reference and README omit live mission-close and loop commands.
- F12: concurrent loop transitions classify before lock and can resurrect a deactivated loop.
- F13: concurrent `task finish` can double-commit and overwrite proof truth.
- F14: review record racing with replan returns `PLAN_INVALID` and drops stale audit event.
- F15: top-level mission truth files are trusted through symlinks outside `PLANS/<mission>`.
- F16: review commands can complete a review before the review task's own DAG dependencies are ready.
- F17: harmless state mutations while a review is open make valid findings audit-only.

## Repair Groups

### 1. Mission Truth Read Containment And Outcome Preflight

Findings: F01, F15.

Intended behavior:
- CLI-owned truth artifacts must not be trusted through symlinks outside the mission.
- `outcome ratify` must fail before mutating state when `OUTCOME.md` is not a safe mission-owned file.

Files to edit:
- `crates/codex1/src/core/paths.rs`
- `crates/codex1/src/state/mod.rs`
- plan/outcome readers that open `STATE.json`, `PLAN.yaml`, or `OUTCOME.md`
- `crates/codex1/src/cli/outcome/ratify.rs`

Tests:
- `foundation.rs`: symlinked `STATE.json` rejected before `status` can trust outside state.
- `plan_check.rs` or `foundation.rs`: symlinked `PLAN.yaml` rejected before lock.
- `outcome.rs`: symlinked `OUTCOME.md` ratify fails with unchanged `STATE.json` and unchanged `EVENTS.jsonl`.

Risks:
- Some commands may currently read scaffold placeholders before files exist. Helpers must distinguish “missing file” from “unsafe existing file” and preserve existing error codes where possible.

### 2. Locked Transactions And Idempotency

Findings: F10, F12, F13, F14.

Intended behavior:
- Mutating commands must classify and validate against the state read under the exclusive lock.
- Stale or rejected writers must not publish truth artifacts.
- Concurrent terminal/finish/loop transitions must produce one committed state transition or a semantic no-op/error.
- Stale planned-review output superseded by a concurrent replan must be audited as stale rather than dropped as `PLAN_INVALID`.

Files to edit:
- `crates/codex1/src/state/mod.rs`
- `crates/codex1/src/cli/close/complete.rs`
- `crates/codex1/src/cli/task/finish.rs`
- `crates/codex1/src/cli/loop_/mod.rs`
- `crates/codex1/src/cli/review/record.rs`

Tests:
- `close.rs`: `CLOSEOUT.md` final revision matches committed state; concurrent `close complete --expect-revision` cannot let rejected writer own `CLOSEOUT.md`; unfenced concurrent close does not append multiple terminal events.
- `task.rs`: concurrent `task finish` emits exactly one finish event and preserves proof truth.
- `loop_.rs`: pause-vs-deactivate and resume-vs-deactivate cannot resurrect active loop after deactivate wins.
- `review.rs` or replan e2e: review record racing with replan produces stale audit event/category instead of `PLAN_INVALID`.

Risks:
- Holding the state lock while writing `CLOSEOUT.md` is deliberate for artifact/state atomicity at the CLI level, but must avoid recursive state lock acquisition.
- Existing `state::mutate` always appends event after mutator returns; close complete may need a specialized locked transaction helper or a general dynamic artifact transaction helper.

### 3. Review Boundary And DAG Readiness

Findings: F16, F17.

Intended behavior:
- A review task may not start/record until the review task itself is DAG-ready, not just its explicit review targets.
- Current open review output must remain accepted-current when unrelated state changes occur and the review task/targets remain the same current boundary.

Files to edit:
- `crates/codex1/src/cli/review/plan_read.rs`
- `crates/codex1/src/cli/review/start.rs`
- `crates/codex1/src/cli/review/record.rs`
- `crates/codex1/src/cli/review/classify.rs`
- possibly `crates/codex1/src/state/schema.rs` if review boundary metadata needs extension

Tests:
- Review task with extra pending dependency cannot `review start`; cannot `review record --clean` around the dependency either.
- Unrelated loop mutation after `review start` does not convert current dirty findings to `late_same_boundary`.
- Preserve accepted stale/superseded and terminal-contamination behavior.

Risks:
- Changing classification semantics must not re-open true late/stale records after target task state/proof changed or replan superseded the target.
- If schema extension is needed, deserialization defaults must preserve existing state files.

### 4. Status, Waves, And Dirty-Repair Handoff

Findings: F02, F03, F11.

Intended behavior:
- `task next`, `status`, and `plan waves` agree on the current executable topological wave.
- `stop.allow` follows the Ralph contract and is not gated by proof-aware close blockers.
- Dirty-review targets are advertised as repair only until they have been repaired; after fresh repair finish, status should route to the ready review.

Files to edit:
- `crates/codex1/src/cli/task/lifecycle.rs`
- `crates/codex1/src/cli/task/next.rs`
- `crates/codex1/src/cli/status/next_action.rs`
- `crates/codex1/src/cli/status/project.rs`
- possibly `crates/codex1/src/state/readiness.rs`

Tests:
- `task.rs`: root pending + other root complete + child pending returns only the current W1 task.
- `status.rs` or `close.rs`: active/unpaused mission-close-passed with missing proof has `close_ready:false`, blocked close next action, and `stop.allow:true`.
- `review.rs`/`status.rs`: dirty review -> repair start/finish -> status no longer says repair and does surface ready review.

Risks:
- `dirty_repair_targets` must distinguish freshly repaired `AwaitingReview` from still-broken `AwaitingReview` without hiding legitimate repair needs. Use timestamps/boundary data conservatively.

### 5. Documentation Flow

Findings: F08, F09.

Intended behavior:
- README and CLI reference examples must be copy-pasteable and match path-resolution contracts.
- CLI reference should include live `loop activate` and `close record-review` subcommands.
- README manual flow should include mission-close review recording before `close complete`.

Files to edit:
- `README.md`
- `docs/cli-reference.md`

Tests:
- No code test strictly required. Existing `make verify-contract` covers CLI install/contract behavior. Consider grep-level docs sanity only if existing patterns support it.

Risks:
- Avoid broad docs rewrites; keep examples compact and aligned with existing style.

## Verification

Run after implementation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
make verify-contract
```

Because this pass touches CLI behavior, status/close behavior, docs, and install-facing examples, `make verify-contract` is required.

