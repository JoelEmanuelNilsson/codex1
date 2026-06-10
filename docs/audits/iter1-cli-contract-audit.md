# CLI Contract Audit — iter1 (post-6473650)

Branch audited: `audit-iter1-wave` (off `main`) @ `6473650`
Audited on: 2026-04-20 UTC
Binary: `target/release/codex1 0.1.0` (built from this tree).

## Build / test evidence (iter1 header)

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS (no output) |
| `cargo clippy --all-targets -- -D warnings` | **FAIL** — see new finding P2-1 below (`crates/codex1/tests/plan_scaffold.rs:38`). |
| `cargo test --release` | PASS — 169 tests across 19 test binaries (steady-state). First run from a clean build flakes 7 tests in `tests/close.rs` because `cargo test --release` does not reliably build the bin crate before running integration tests that use `Command::cargo_bin("codex1")`; second and third runs are clean. Evidence captured under "Clean-checks § Test suite flake" below. |

The commit message of `6473650` claims "cargo fmt + cargo clippy --all-targets -- -D warnings clean." Clippy is not clean; the claim is falsifiable on the same tree.

## Summary

**0 P0, 0 P1, 1 P2.** The five baseline `cli-contract-audit.md` findings are all fixed (verified live). The fresh pass surfaces one new P2 issue: a clippy warning that breaks the `-D warnings` gate promised by the commit message.

## Re-verification of the 5 baseline findings

### P1-1 (baseline): `plan choose-level` must return `OUTCOME_NOT_RATIFIED` when OUTCOME.md is unratified — **FIXED**

Live-verified in `/tmp/codex1-iter1-p1-1`:

```bash
$ codex1 --json init --mission t1       # ok:true, outcome.ratified=false
$ codex1 --json plan choose-level --mission t1 --level medium
{
  "ok": false,
  "code": "OUTCOME_NOT_RATIFIED",
  "message": "OUTCOME.md is not ratified",
  "retryable": false
}
```

Source: `crates/codex1/src/cli/plan/choose_level.rs:19-40` now loads STATE.json and returns `Err(CliError::OutcomeNotRatified)` before the mutation. Matches `docs/cli-reference.md:98`.

### P1-2 (baseline): `close record-review` and `loop activate` listed in handoff + per-command schemas — **FIXED**

- `docs/codex1-rebuild-handoff/02-cli-contract.md:55-94` § Minimal Command Surface lists `loop activate` (line 89) and `close record-review` (line 93). Explanatory paragraphs on lines 96-104 describe each.
- `docs/cli-contract-schemas.md:320-337` documents `close record-review --clean|--findings-file`; `docs/cli-contract-schemas.md:339-346` documents `loop activate --mode`.

Clap wiring confirmed: `codex1 loop --help` shows `activate` first; `codex1 close --help` shows `record-review` as a peer of `check` and `complete`.

### P2-1 (baseline): `CliError::Other` / `"INTERNAL"` removed — **FIXED**

- Grep `CliError::Other` across `crates/codex1/src/**/*.rs`: 0 matches.
- Grep `"INTERNAL"` across `crates/codex1/src/**/*.rs`: 0 matches as a code string. The only hit is a doc comment at `crates/codex1/src/core/config.rs:29` ("error set stays closed — there is no `INTERNAL` escape hatch"), which is the positive framing of the removal.
- `crates/codex1/src/core/error.rs:21-76` enumerates the canonical `CliError` variants; no `Other(anyhow::Error)` variant exists. Every variant's `code()` string (lines 80-103) is in `docs/cli-contract-schemas.md:46-63`.
- `crates/codex1/src/core/config.rs:26-44` now returns `CliError::ParseError` / `CliError::ConfigMissing` directly rather than propagating `anyhow::Error`.

### P2-2 (baseline): Error envelope `hint`/`context` documented as optional-when-null — **FIXED**

`docs/cli-contract-schemas.md:36-40`:

> **Optional fields.** `hint` and `context` are omitted from the serialized envelope when they are null / empty — the CLI uses `#[serde(skip_serializing_if = "…")]` for both. Callers MUST treat missing `hint`/`context` as equivalent to the empty value. `ok`, `code`, `message`, and `retryable` are always present.

Matches the observed envelope shape (`crates/codex1/src/core/envelope.rs:72-77`).

### P2-3 (baseline): `Commands::Doctor` doc comment expanded — **FIXED**

`crates/codex1/src/cli/mod.rs:77-82`:

```rust
/// Report CLI health as a stable JSON envelope: `version`, `config`
/// path + existence, `install` (binary-on-PATH + `~/.local/bin`
/// writability), `auth` (always `required: false`), `cwd`, and any
/// `warnings`. Never crashes on missing auth or config — returns
/// `{ok: true, ...}` and surfaces problems as warnings instead.
Doctor,
```

Surfaces `version`, `config`, `install`, `auth`, `cwd`, `warnings` — all the real envelope fields — instead of the one-liner flagged in the baseline audit.

## Re-verification of e2e-walkthrough fixes relevant to the contract

Although the task does not ask for a separate e2e re-audit, the E2E fixes touch CLI-contract surfaces. A fresh mission walkthrough under `/tmp/codex1-iter1-e2e` re-exercised all seven fixes:

| Baseline finding | Fix location | Re-verified in walkthrough step |
| --- | --- | --- |
| F1 — status.next_action and task next disagreed after T2/T3 → AwaitingReview | `crates/codex1/src/cli/status/next_action.rs:159-175` (AwaitingReview excluded from ready set for non-review kinds) | After T3 finish: `task next` returned `run_review T4 targets:[T2,T3]` AND `status.next_action` returned `run_review T4 targets:[T2,T3]`, `ready_tasks: ["T4"]`. Agreement. |
| F2 — status.review_required did not consult state.reviews | `crates/codex1/src/cli/status/next_action.rs:102-108` (`ready_reviews` filters out reviews with Clean verdict) | After T4 recorded clean: `status.review_required: []`. After mission close: still empty. |
| F3 — review record clean did not create a TaskRecord for the review task | `crates/codex1/src/cli/review/record.rs:286-340` (clean record now upserts TaskRecord with `status: complete`) | After T4 clean: `codex1 task status T4` → `{ status: "complete", kind: "review", deps_status: {T2: complete, T3: complete} }`. |
| F4 — review packet envelope renamed `target_proofs` → `proofs` | `crates/codex1/src/cli/review/packet.rs` | `codex1 review packet T4` data keys now include `proofs` (not `target_proofs`). Verified both positively and negatively. |
| F5 — review-record envelope extras documented | `docs/cli-contract-schemas.md:274-291` | Documented `findings_file`, `replan_triggered`, `warnings` as additive. |
| F6 — task-finish proof path resolution documented | `docs/cli-contract-schemas.md:83-98` (new "Per-command relative path resolution" section) | Resolver rule stated; contrasts with `review record`/`close record-review` which resolve relative to CWD. |
| F7 — CLOSEOUT.md Tasks table omitted T4 | Downstream of F3 | T4 now appears in the Tasks table of the generated CLOSEOUT.md (verified on iter1 walkthrough CLOSEOUT at `/tmp/codex1-iter1-e2e/PLANS/demo/CLOSEOUT.md`). |

## Findings

### P0: none.

### P1: none.

### P2-1 (new, only): `cargo clippy --all-targets -- -D warnings` fails at `crates/codex1/tests/plan_scaffold.rs:38`

- **Severity:** P2.
- **Where:** `crates/codex1/tests/plan_scaffold.rs:38` uses `let outcome = r#"---"#;` (raw string with hashes) where `r"..."` suffices because the string contains no inner `"#`. Clippy's `needless_raw_string_hashes` lint (stable-default in the `rust-clippy 1.94.0` toolchain in use) rejects this under `-D warnings`.
- **Observed:**
  ```bash
  $ cargo clippy --all-targets -- -D warnings
  error: unnecessary hashes around raw string literal
    --> crates/codex1/tests/plan_scaffold.rs:38:19
     |
  38 |       let outcome = r#"---
  ... (67 lines elided)
  68 | | "#;
     |
     = note: `-D clippy::needless-raw-string-hashes` implied by `-D warnings`

  error: could not compile `codex1` (test "plan_scaffold") due to 1 previous error
  ```
- **Expected:** Clippy passes with `-D warnings`. The commit message of `6473650` states "cargo fmt + cargo clippy --all-targets -- -D warnings clean," which is falsifiable on the same tree. Either the raw string should be `r"---\n...\n"` (drop the `#` hashes), or the lint should be `#[allow]`-ed with a rationale.
- **Contract reference:** Not a CLI-schema issue; a build-gate claim issue. The commit message creates a promise (the `-D warnings` gate is passing); re-verification shows it isn't. Left at P2 because no runtime behavior changes, but the gate-claim mismatch is real.
- **Fix sketch:** Replace `r#"---\n...\n"#` with `r"---\n...\n"` (no hashes). Or add `#[allow(clippy::needless_raw_string_hashes)]` above `fn seed_valid_outcome` with a one-line rationale.
- **Scope note:** The task explicitly forbids modifying existing test files; I did not apply the fix, only reported it.

## Clean-checks (no findings)

### Minimal command surface

All 25 verbs from `docs/codex1-rebuild-handoff/02-cli-contract.md:55-94` resolve via clap:

| Verb | clap site |
| --- | --- |
| `codex1 init` | `crates/codex1/src/cli/mod.rs:76`, `init.rs` |
| `codex1 status` | `cli/mod.rs:124`, `status/mod.rs` |
| `codex1 outcome check` / `ratify` | `outcome/mod.rs` |
| `codex1 plan choose-level` / `scaffold` / `check` / `graph` / `waves` | `plan/mod.rs` |
| `codex1 task next` / `start` / `finish` / `status` / `packet` | `task/mod.rs` |
| `codex1 review start` / `packet` / `record` / `status` | `review/mod.rs` |
| `codex1 replan record` / `check` | `replan/mod.rs` |
| `codex1 loop pause` / `resume` / `deactivate` / `activate` | `loop_/mod.rs` |
| `codex1 close check` / `complete` / `record-review` | `close/mod.rs` |

Verified live by walking `codex1 <group> --help` for each group.

### Every error code in `crates/codex1/src/**/*.rs` is canonical

- `CliError` variant codes (`crates/codex1/src/core/error.rs:80-103`): `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`, `CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`, `STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`, `CONFIG_MISSING`, `MISSION_NOT_FOUND`, `PARSE_ERROR`, `NOT_IMPLEMENTED`. All 18 appear in the canonical table at `docs/cli-contract-schemas.md:46-63`.
- Raw-string codes constructed in handler source:
  - `cli/plan/check.rs:504-515` (`exit_with_validation_error`) uses only `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`.
  - `cli/close/check.rs` `Blocker::new(...)` sites use only `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `REPLAN_REQUIRED`, `TASK_NOT_READY`, `REVIEW_FINDINGS_BLOCK`, `CLOSE_NOT_READY`.
- No handler constructs a bespoke `JsonErr` with a non-canonical code.

### `status` ↔ `close check` agreement across 6 synthetic STATE.json fixtures

Six hand-authored STATE.json fixtures were dropped into `/tmp/codex1-iter1-fix-<N>/PLANS/m1/STATE.json` (with the same locked PLAN.yaml + OUTCOME.md), then `codex1 status` and `codex1 close check` were run against each.

| Fixture | `status.verdict` | `status.close_ready` | `close check.verdict` | `close check.ready` |
| --- | --- | --- | --- | --- |
| 1. Fresh init (outcome unratified, plan unlocked) | `needs_user` | `false` | `needs_user` | `false` |
| 2. Plan locked, T1 ready, no tasks started | `continue_required` | `false` | `continue_required` | `false` |
| 3. All tasks complete, T2 review clean, mission-close not started | `ready_for_mission_close_review` | `false` | `ready_for_mission_close_review` | `false` |
| 4. Mission-close review passed | `mission_close_review_passed` | `true` | `mission_close_review_passed` | `true` |
| 5. Terminal (`close.terminal_at` set) | `terminal_complete` | `false` | `terminal_complete` | `false` |
| 6. Replan triggered (`replan.triggered: true`) | `blocked` | `false` | `blocked` | `false` |

`status.close_ready` equals `close check.ready` in every row. Both surfaces derive through `crates/codex1/src/state/readiness.rs::derive_verdict`, so disagreement would be a logic error in one of two consumers — none was observed.

### Test suite flake (informational, not a finding)

On a cold target cache, `cargo test --release` has been observed to fail 7 tests in `tests/close.rs` because `Command::cargo_bin("codex1")` looked up the release binary before cargo finished building the bin crate. Re-running `cargo test --release` without other changes yields 169 / 169 passing. Not a product bug; not a contract issue; documented here so the next reviewer can distinguish flake from regression.

### Stable JSON envelopes

- Success envelope (`crates/codex1/src/core/envelope.rs:20-29`): `{ ok: true, mission_id?, revision?, data }`. Verified live on `doctor`, `init`, `outcome ratify`, `plan choose-level`, `plan scaffold`, `plan check`, `plan waves`, `plan graph`, `task next`, `task start`, `task finish`, `review start`, `review packet`, `review record`, `close check`, `close record-review`, `close complete`, `status`.
- Error envelope (`core/envelope.rs:67-78`): `{ ok: false, code, message, hint?, retryable, context? }`. Verified live on `plan choose-level` (OUTCOME_NOT_RATIFIED), `plan check` (PLAN_INVALID), `close check` (CLOSE_NOT_READY via blockers), `close complete` (TERMINAL_ALREADY_COMPLETE on re-run).

### Global flags

`--mission <ID>`, `--repo-root <PATH>`, `--json`, `--dry-run`, `--expect-revision <N>` are globals on every subcommand per `cli/mod.rs:43-69`. `codex1 <cmd> --help` renders all five on every leaf.

## Reading map

- `crates/codex1/src/cli/plan/choose_level.rs:19-40` — P1-1 fix landing site.
- `docs/codex1-rebuild-handoff/02-cli-contract.md:55-104` — P1-2 fix landing site (handoff).
- `docs/cli-contract-schemas.md:320-346` — P1-2 fix landing site (schemas).
- `crates/codex1/src/core/error.rs`, `crates/codex1/src/core/config.rs:26-44` — P2-1 landing sites.
- `docs/cli-contract-schemas.md:36-40` — P2-2 landing site.
- `crates/codex1/src/cli/mod.rs:77-82` — P2-3 landing site.
- `crates/codex1/tests/plan_scaffold.rs:38` — new P2-1 (iter1) finding.
