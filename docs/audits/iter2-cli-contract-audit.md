# Iter2 CLI Contract Audit

Branch audited: `audit-iter2-wave` (worktree off `main` @ `271b2fc`)
Audit iteration: iter 2 (after iter1 fix at `6473650` and the clippy follow-up at `271b2fc`).
Audited on: 2026-04-20 UTC.
Binary: `target/release/codex1 0.1.0` (built from the audited tree).

## Build evidence

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS (no diff) |
| `cargo clippy --all-targets -- -D warnings` | PASS (no warnings, no errors) |
| `cargo test --release` | PASS — 169 passed / 0 failed / 0 ignored across 18 test binaries + 0 doc tests |

## Scope

Re-ran the entire CLI contract audit from scratch against commit `271b2fc`:

1. Enumerated every command in `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface (27 verbs + the implicit `init`/`status` totals) and confirmed each is wired in clap and exposes `--help`.
2. Verified envelope shape (`ok`, optional `mission_id`, optional `revision`, `data` for success; `ok`, `code`, `message`, optional `hint`, `retryable`, optional `context` for errors) end-to-end.
3. Walked every error-code string under `crates/codex1/src/**/*.rs` and confirmed each is a documented `CliError` variant.
4. Built 7 synthetic STATE.json fixtures and compared `codex1 status --json` against `codex1 close check --json` for `verdict` + `close_ready`/`ready`.
5. Confirmed every clap subcommand group is documented in either `docs/cli-reference.md` or `docs/cli-contract-schemas.md`.

## Summary

**0 P0 / 0 P1 / 0 P2.** Tree is clean. Every iter1 fix is in place at `271b2fc`; no new drift introduced.

## Findings

No findings.

## Clean checks (every check passed)

### CC-1: Minimal Command Surface — all 27 verbs wired in clap, all expose `--help`

The handoff minimal surface (`docs/codex1-rebuild-handoff/02-cli-contract.md:59-94`) lists 27 verbs after the iter1 fix added `loop activate` (line 89) and `close record-review` (line 93). All 27 are wired and reach a leaf clap command:

| Verb | clap location | `--help` works |
| --- | --- | --- |
| `init` | `cli/mod.rs:76` → `cli/init.rs:19` | yes |
| `status` | `cli/mod.rs:124` → `cli/status/mod.rs` | yes |
| `outcome check` | `cli/outcome/mod.rs` | yes |
| `outcome ratify` | `cli/outcome/mod.rs` | yes |
| `plan choose-level` | `cli/plan/mod.rs` | yes |
| `plan scaffold` | `cli/plan/mod.rs` | yes |
| `plan check` | `cli/plan/mod.rs` | yes |
| `plan graph` | `cli/plan/mod.rs` | yes |
| `plan waves` | `cli/plan/mod.rs` | yes |
| `task next` | `cli/task/mod.rs` | yes |
| `task start` | `cli/task/mod.rs` | yes |
| `task finish` | `cli/task/mod.rs` | yes |
| `task status` | `cli/task/mod.rs` | yes |
| `task packet` | `cli/task/mod.rs` | yes |
| `review start` | `cli/review/mod.rs` | yes |
| `review packet` | `cli/review/mod.rs` | yes |
| `review record` | `cli/review/mod.rs` | yes |
| `review status` | `cli/review/mod.rs` | yes |
| `replan check` | `cli/replan/mod.rs` | yes |
| `replan record` | `cli/replan/mod.rs` | yes |
| `loop activate` | `cli/loop_/mod.rs:27` → `cli/loop_/activate.rs` | yes |
| `loop pause` | `cli/loop_/mod.rs:32` | yes |
| `loop resume` | `cli/loop_/mod.rs:34` | yes |
| `loop deactivate` | `cli/loop_/mod.rs:36` | yes |
| `close check` | `cli/close/mod.rs:46` → `cli/close/check.rs` | yes |
| `close complete` | `cli/close/mod.rs:48` → `cli/close/complete.rs` | yes |
| `close record-review` | `cli/close/mod.rs:50-60` → `cli/close/record_review.rs` | yes |

The two extra surfaces beyond the minimal set — `doctor` (`cli/mod.rs:82`) and `hook snippet` (`cli/hook.rs:13-15`) — are documented in `docs/cli-reference.md:30-56` and called out in `docs/cli-contract-schemas.md:71` ("Optional for `doctor`, `hook snippet`."). Both are explicitly informational (no mission binding required) and exempt from the minimal-surface "Do not add more commands" rule because they predate it as Foundation utilities.

`--help` was verified live for every leaf:

```bash
$ ./target/release/codex1 outcome check --help          # OK (Usage: codex1 outcome check ...)
$ ./target/release/codex1 plan choose-level --help      # OK
$ ./target/release/codex1 plan scaffold --help          # OK
$ ./target/release/codex1 plan check --help             # OK
$ ./target/release/codex1 plan graph --help             # OK
$ ./target/release/codex1 plan waves --help             # OK
$ ./target/release/codex1 task next --help              # OK
$ ./target/release/codex1 task start --help             # OK
$ ./target/release/codex1 task finish --help            # OK
$ ./target/release/codex1 task status --help            # OK
$ ./target/release/codex1 task packet --help            # OK
$ ./target/release/codex1 review start --help           # OK
$ ./target/release/codex1 review packet --help          # OK
$ ./target/release/codex1 review record --help          # OK
$ ./target/release/codex1 review status --help          # OK
$ ./target/release/codex1 replan check --help           # OK
$ ./target/release/codex1 replan record --help          # OK
$ ./target/release/codex1 loop activate --help          # OK
$ ./target/release/codex1 loop pause --help             # OK
$ ./target/release/codex1 loop resume --help            # OK
$ ./target/release/codex1 loop deactivate --help        # OK
$ ./target/release/codex1 close check --help            # OK
$ ./target/release/codex1 close complete --help         # OK
$ ./target/release/codex1 close record-review --help    # OK
$ ./target/release/codex1 outcome ratify --help         # OK
$ ./target/release/codex1 hook snippet --help           # OK
```

(All 26 leaves print a `Usage: codex1 …` line and the global flag block.)

### CC-2: Stable JSON envelopes — success and error shape verified live

**Success envelope** (`crates/codex1/src/core/envelope.rs:20-65`):

```rust
pub struct JsonOk {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub data: Value,
}
```

Verified live:

```bash
$ codex1 init --mission demo --repo-root <work>
{ "ok": true, "mission_id": "demo", "revision": 0, "data": { ... } }

$ codex1 status --mission f5 --repo-root <fixture-work>
{ "ok": true, "mission_id": "f5", "revision": 14, "data": { ..., "verdict": "mission_close_review_passed", "close_ready": true, ... } }

$ codex1 doctor                              # global success (no mission_id, no revision)
{ "ok": true, "data": { "version": "0.1.0", ... } }
```

**Error envelope** (`crates/codex1/src/core/envelope.rs:67-78`):

```rust
pub struct JsonErr {
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub context: Value,
}
```

Verified live:

```bash
$ codex1 close complete --mission f1 --repo-root <fixture>
{ "ok": false, "code": "CLOSE_NOT_READY", "message": "...", "retryable": false }
# (no `hint`, no `context` — both omitted because Option::is_none / Value::is_null)

$ codex1 plan choose-level --level medium --mission test1 --repo-root <work>
{ "ok": false, "code": "OUTCOME_NOT_RATIFIED", "message": "OUTCOME.md is not ratified", "retryable": false }

$ codex1 status --mission nope --repo-root <work>
{ "ok": false, "code": "MISSION_NOT_FOUND", "message": "...", "hint": "Run `codex1 init --mission <id>` first, or pass --mission/--repo-root.", "retryable": false }

$ codex1 loop pause --mission f2 --repo-root <fixture> --expect-revision 999
{ "ok": false, "code": "REVISION_CONFLICT", "message": "...", "hint": "...", "retryable": true,
  "context": { "actual": 5, "expected": 999 } }

$ codex1 replan record --reason fake_reason --mission f2 --repo-root <fixture>
{ "ok": false, "code": "PLAN_INVALID", "message": "...",
  "hint": "Use one of: six_dirty, scope_change, architecture_shift, risk_discovered, user_request.",
  "retryable": false }
```

The two optional fields obey `docs/cli-contract-schemas.md:36-40`: callers MUST treat missing `hint`/`context` as equivalent to the empty value. `ok`, `code`, `message`, and `retryable` are always present in every error envelope produced by the live binary.

### CC-3: Every error-code string is a documented `CliError` variant

`CliError::code()` (`crates/codex1/src/core/error.rs:78-103`) maps every variant to one of these 18 canonical codes:

```text
OUTCOME_INCOMPLETE     OUTCOME_NOT_RATIFIED   PLAN_INVALID
DAG_CYCLE              DAG_MISSING_DEP        TASK_NOT_READY
PROOF_MISSING          REVIEW_FINDINGS_BLOCK  REPLAN_REQUIRED
CLOSE_NOT_READY        STATE_CORRUPT          REVISION_CONFLICT
STALE_REVIEW_RECORD    TERMINAL_ALREADY_COMPLETE   CONFIG_MISSING
MISSION_NOT_FOUND      PARSE_ERROR            NOT_IMPLEMENTED
```

This matches the canonical set in `docs/cli-contract-schemas.md:42-63` exactly (1:1, no extras, no missing). Confirmed by grep over `crates/codex1/src/`:

- `crates/codex1/src/cli/plan/check.rs` — 28 raw-string code constructions, all `PLAN_INVALID` / `DAG_CYCLE` / `DAG_MISSING_DEP` (lines 45, 144, 155, 165, 174, 183, 192, 201, 217, 225, 248, 256, 289, 317, 325, 339, 360, 368, 377, 386, 397, 415, 428, 438, 447, 468, 479, 491). All canonical.
- `crates/codex1/src/cli/close/check.rs:80,85,93,98,106,115,121` — `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `REPLAN_REQUIRED`, `TASK_NOT_READY`, `REVIEW_FINDINGS_BLOCK`, `CLOSE_NOT_READY`. All canonical.
- No bespoke `JsonErr::new(<non-canonical>, ...)` exists. The only two `JsonErr::new(...)` call sites (`crates/codex1/src/cli/plan/check.rs:504`, `crates/codex1/src/core/error.rs:169`) both receive a canonical code (the `plan/check.rs` site is gated by `exit_with_validation_error` which is only ever called with one of the three plan codes above).
- No `Other` / `anyhow::Error` / `INTERNAL` variants exist in `CliError`. The iter1 fix removed `CliError::Other` and `impl From<anyhow::Error>`; grep for `Other(anyhow|anyhow::Error|impl From<anyhow` across `crates/` confirms zero matches.

### CC-4: `status` ↔ `close check` agreement across 7 synthetic STATE.json fixtures

Wrote 7 hand-crafted STATE.json fixtures, ran `codex1 status --mission <id> --repo-root <work>` and `codex1 close check --mission <id> --repo-root <work>` against the release binary. Both commands derive their verdict from `state::readiness::derive_verdict` (`crates/codex1/src/state/readiness.rs:40-66`) — `status` calls it from `cli/status/project.rs:19` and `close check` from `cli/close/check.rs:47`.

| Fixture | State summary | `status.verdict` | `status.close_ready` | `close check.verdict` | `close check.ready` | Agree |
| --- | --- | --- | --- | --- | --- | --- |
| f1 | Fresh init (outcome unratified, plan unlocked) | `needs_user` | `false` | `needs_user` | `false` | yes |
| f2 | Outcome ratified, plan locked, T1 in progress | `continue_required` | `false` | `continue_required` | `false` | yes |
| f3 | T1 complete, T2 in progress, T2 review dirty | `blocked` | `false` | `blocked` | `false` | yes |
| f4 | All tasks complete, mission-close NotStarted | `ready_for_mission_close_review` | `false` | `ready_for_mission_close_review` | `false` | yes |
| f5 | All tasks complete, mission-close Passed | `mission_close_review_passed` | `true` | `mission_close_review_passed` | `true` | yes |
| f6 | Terminal (`close.terminal_at` set) | `terminal_complete` | `false` | `terminal_complete` | `false` | yes |
| f7 | `replan.triggered = true` (six_consecutive_dirty) | `blocked` | `false` | `blocked` | `false` | yes |

`close_ready` (status) and `ready` (close check) are both `true` iff `verdict == mission_close_review_passed`. This matches `state::readiness::close_ready` (`readiness.rs:69-72`) and `ReadinessReport::ready` (`cli/close/check.rs:49`). The two predicates are the same predicate.

This is corroborated by `crates/codex1/tests/status_close_agreement.rs`, which runs the same agreement check across 20 in-process fixtures plus a CLI-spawned `close check` smoke test. Both tests pass under `cargo test --release` (3 passed, including `status_agrees_with_readiness_helpers_for_all_fixtures`).

### CC-5: Envelope shape — `hint`/`context` optionality matches `cli-contract-schemas.md`

The schema doc at `docs/cli-contract-schemas.md:36-40` says:

> **Optional fields.** `hint` and `context` are omitted from the serialized envelope when they are null / empty — the CLI uses `#[serde(skip_serializing_if = "…")]` for both. Callers MUST treat missing `hint`/`context` as equivalent to the empty value. `ok`, `code`, `message`, and `retryable` are always present.

Verified against `crates/codex1/src/core/envelope.rs:73-77`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub hint: Option<String>,
pub retryable: bool,
#[serde(skip_serializing_if = "Value::is_null")]
pub context: Value,
```

Live verification across both branches (with hint/context, without hint/context) — see CC-2 above. Documentation and implementation are in sync.

### CC-6: No undocumented commands in clap

`crates/codex1/src/cli/mod.rs:74-125` declares the top-level `Commands` enum. Eleven variants:

```text
Init  Doctor  Hook  Outcome  Plan  Task  Review  Replan  Loop  Close  Status
```

Coverage:

- 9 of 11 (everything except `Doctor` and `Hook`) appear in the handoff minimal surface and are documented in `docs/cli-reference.md` + `docs/cli-contract-schemas.md`.
- `Doctor` is documented in `docs/cli-reference.md:30-40` and called out at `docs/cli-contract-schemas.md:71`.
- `Hook` (with one subvariant `Snippet`) is documented in `docs/cli-reference.md:44-56` and `docs/cli-contract-schemas.md:71`.

Subcommand variants:

- `OutcomeCmd::{Check, Ratify}` — documented at `docs/cli-reference.md:60-86`.
- `PlanCmd::{ChooseLevel, Scaffold, Check, Graph, Waves}` — documented at `docs/cli-reference.md:90-169`.
- `TaskCmd::{Next, Start, Finish, Status, Packet}` — documented at `docs/cli-reference.md:173-249`.
- `ReviewCmd::{Start, Packet, Record, Status}` — documented at `docs/cli-reference.md:253-317`.
- `ReplanCmd::{Check, Record}` — documented at `docs/cli-reference.md:321-349`.
- `LoopCmd::{Activate, Pause, Resume, Deactivate}` — `Pause/Resume/Deactivate` documented at `docs/cli-reference.md:353-394`. `Activate` is documented in `docs/cli-contract-schemas.md:339-346` (per the handoff "documented in `docs/cli-reference.md` or `docs/cli-contract-schemas.md`").
- `CloseCmd::{Check, Complete, RecordReview}` — `Check/Complete` documented at `docs/cli-reference.md:398-424`. `RecordReview` is documented in `docs/cli-contract-schemas.md:320-337`.
- `HookCmd::{Snippet}` — documented at `docs/cli-reference.md:44-56`.

Every clap subcommand has a documentation home. Nothing in the clap tree is undocumented.

### CC-7: Global flags consistently honored

Every command exposes the five global flags declared in `cli/mod.rs:43-66`: `--mission <ID>`, `--repo-root <PATH>`, `--json`, `--dry-run`, `--expect-revision <N>`. Verified by inspecting each `--help` output (CC-1 above).

Mission resolution per `docs/cli-contract-schemas.md:76-81` is implemented in `core/mission::resolve_mission` (the file). The graceful no-mission `codex1 status` envelope is verified live:

```bash
$ cd /tmp && codex1 status
{ "ok": true, "data": { "foundation_only": false, "stop": { "allow": true, "reason": "no_mission", ... }, "verdict": "needs_user" } }
```

Ralph never receives a non-zero exit just because no mission resolves.

### CC-8: `plan choose-level` honors the documented `OUTCOME_NOT_RATIFIED` gate

`crates/codex1/src/cli/plan/choose_level.rs:25-28` loads state and short-circuits with `Err(CliError::OutcomeNotRatified)` before any mutation, before the interactive prompt, before reading `--level`. Verified live:

```bash
$ codex1 init --mission test1 --repo-root <work>
$ codex1 plan choose-level --level medium --mission test1 --repo-root <work>
{ "ok": false, "code": "OUTCOME_NOT_RATIFIED",
  "message": "OUTCOME.md is not ratified", "retryable": false }
```

This is the iter1 P1-1 fix; verified intact at `271b2fc`.

### CC-9: `state::mutate` revision protocol honored

Every mutating CLI handler routes through `state::mutate` (or `state::init_write` for `init`), which (per `docs/cli-contract-schemas.md:134-153`) acquires an exclusive fs2 lock, validates `--expect-revision` if supplied, runs the closure, bumps `revision` and `events_cursor`, atomically writes `STATE.json`, and appends one event. Verified live:

```bash
$ codex1 loop pause --mission f2 --repo-root <fixture> --expect-revision 999
{ "ok": false, "code": "REVISION_CONFLICT", "message": "Revision conflict (expected 999, actual 5)",
  "hint": "Re-read STATE.json and retry with --expect-revision 5 (you sent 999).",
  "retryable": true, "context": { "actual": 5, "expected": 999 } }
```

`REVISION_CONFLICT` is the only `retryable: true` variant in `CliError::retryable()` (`core/error.rs:106-108`), matching the schemas table.

### CC-10: `doctor` returns the documented success shape

`crates/codex1/src/cli/doctor.rs:10-42` emits `{version, config: {path, exists}, install: {codex1_on_path, home_local_bin, home_local_bin_writable}, auth: {required: false, notes}, cwd, warnings}`. The `Commands::Doctor` doc comment at `cli/mod.rs:77-82` describes every field, satisfying the iter1 P2-3 documentation expansion.

### CC-11: `hook snippet` returns the install one-liner

`crates/codex1/src/cli/hook.rs:24-50` emits `{hook: {event, script_path_hint, behavior}, install: {codex_hooks_json_example}, note}`. The Ralph hook script lives at `scripts/ralph-stop-hook.sh` (60 lines) and runs exactly one CLI command (`codex1 status --json`). Documented in `docs/cli-reference.md:44-56`.

## Notes (informational, not findings)

- `loop activate` and `close record-review` are documented in `docs/cli-contract-schemas.md` (lines 339, 320) but not in `docs/cli-reference.md`. The audit criterion accepts either ("documented in `docs/cli-reference.md` or `docs/cli-contract-schemas.md`"), so this is not a finding. A future doc pass may want to mirror them into `cli-reference.md` for consistency, but iter1 explicitly chose the schemas-only branch of the fix.
