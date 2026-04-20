# CLI Contract Audit

Branch audited: `integration/phase-b` @ `a9e9abc`
Audited on: 2026-04-20 UTC
Binary: `target/release/codex1 0.1.0` (built from the audited tree).

## Scope

Every command listed in `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface was exercised against the compiled binary. For each, I verified:

- Presence in the clap dispatch tree rooted at `crates/codex1/src/cli/mod.rs`.
- `--help` output.
- Stable JSON envelope shape (success and error).
- Error codes against the canonical `CliError` set in `crates/codex1/src/core/error.rs`.
- `status` ↔ `close check` agreement on `verdict` / `close_ready` / `ready` across seven synthetic states.

## Summary

0 P0, 2 P1, 3 P2. The contract is materially honored: every minimal-surface command is wired, error envelopes match the canonical shape, and `status` and `close check` share `state::readiness` so they cannot disagree. The two P1s are documentation-vs-implementation drift inside Phase B; the three P2s are schema-drift issues that should be absorbed into `docs/cli-contract-schemas.md`.

## Findings

### P1-1: `plan choose-level` does not enforce `OUTCOME_NOT_RATIFIED`

- **Severity:** P1
- **Where:** `crates/codex1/src/cli/plan/choose_level.rs:19-81` (no ratification check before mutating `plan.*_level`); contradicts `docs/cli-reference.md:98` ("**Errors:** `OUTCOME_NOT_RATIFIED`, `MISSION_NOT_FOUND`").
- **Observed:** Against a freshly-`init`'d mission (`outcome.ratified == false`), `codex1 plan choose-level --level medium --mission <id>` returns `ok: true` and mutates `STATE.json` to record `plan.requested_level = Medium` and `plan.effective_level = Medium`. Verified live:
  ```bash
  $ codex1 init --mission test1
  $ codex1 plan choose-level --level medium --mission test1
  { "ok": true, ..., "data": { "requested_level": "medium", ... } }
  ```
- **Expected:** Either (a) reject with `OUTCOME_NOT_RATIFIED` per `docs/cli-reference.md:98`, or (b) drop the documented error from the reference so the contract stops advertising a gate the code does not provide.
- **Contract reference:** `docs/cli-reference.md:98`. The authoritative schemas (`docs/cli-contract-schemas.md`) do not list a ratification gate for this command, so this is a Phase-B-internal doc/impl drift rather than a handoff-level contract break — hence P1, not P0.
- **Fix sketch:** Load state early, short-circuit with `Err(CliError::OutcomeNotRatified)` when `!state.outcome.ratified`, before either the interactive prompt or the mutation. Alternatively amend `docs/cli-reference.md:98`.

### P1-2: `close record-review` and `loop activate` are not listed in the handoff's minimal surface

- **Severity:** P1
- **Where:**
  - `crates/codex1/src/cli/close/mod.rs:44-61` adds `CloseCmd::RecordReview`.
  - `crates/codex1/src/cli/loop_/mod.rs:24-38` adds `LoopCmd::Activate`.
  - `docs/codex1-rebuild-handoff/02-cli-contract.md:59-92` defines the minimal surface, which names only `loop pause|resume|deactivate` and `close check|complete`.
- **Observed:** Both commands ship in the clap tree and are exercised by the skills (`.codex/skills/review-loop/SKILL.md:45` calls `codex1 close record-review`; `crates/codex1/src/cli/loop_/activate.rs:1-3` documents its role as a shared entry point for other mutating handlers). They fill real gaps — `MissionCloseReviewState::Passed` has no other write path, and `loop.active = true` has no other entry point — but they are invisible to any agent reading the handoff contract.
- **Expected:** Either (a) add both to `docs/codex1-rebuild-handoff/02-cli-contract.md` § Minimal Command Surface and to the per-command shape sections of `docs/cli-contract-schemas.md`, or (b) fold `record-review` into `review record <mission-close-target>` and move loop activation behind ratify/check internal state mutations so the surface stays minimal.
- **Contract reference:** `docs/codex1-rebuild-handoff/02-cli-contract.md:59-92` ("Do not add more commands until the minimal set is excellent.").
- **Fix sketch:** Prefer (a) — document them in `docs/cli-contract-schemas.md` with data shapes, error codes, and the `STATE.json` fields each mutates. Rename `close record-review` if the review-record wording should stay unified.

### P2-1: `CliError::Other → "INTERNAL"` is an undocumented error code

- **Severity:** P2
- **Where:** `crates/codex1/src/core/error.rs:77,104` emits `"INTERNAL"` for `CliError::Other(anyhow)`; `docs/cli-contract-schemas.md:36-57` canonical error-code table does not include `INTERNAL`.
- **Observed:** The variant is unreachable from the live handlers (the only `anyhow`-returning helper, `crates/codex1/src/core/config.rs:26-33`, is referenced by `crates/codex1/src/cli/doctor.rs:6` only for `default_config_path()`, not `load()`). But a future handler that `?`-propagates an `anyhow::Result` would silently produce a `code: "INTERNAL"` envelope outside the documented error set.
- **Expected:** Either list `INTERNAL` in the canonical table or delete `CliError::Other` and `impl From<anyhow::Error>` so that schema drift is impossible.
- **Contract reference:** `docs/cli-contract-schemas.md:36-57`.
- **Fix sketch:** Remove the `Other(anyhow::Error)` variant (no caller relies on it). Move `config::load` off `anyhow::Error` onto `CliError`.

### P2-2: Error envelopes omit optional `hint` / `context` fields when null

- **Severity:** P2
- **Where:** `crates/codex1/src/core/envelope.rs:72-77` uses `skip_serializing_if = "Option::is_none"` for `hint` and `skip_serializing_if = "Value::is_null"` for `context`.
- **Observed:** Some error envelopes omit keys that the schema example in `docs/cli-contract-schemas.md:25-34` shows present:
  ```bash
  $ codex1 close complete --mission test1
  { "ok": false, "code": "CLOSE_NOT_READY", "message": "…", "retryable": false }
  # no `hint`, no `context`
  ```
- **Expected:** Either (a) update `docs/cli-contract-schemas.md` § JSON envelopes § Error to note the two fields are optional and absent when null, or (b) always serialize both fields (use `""` / `{}` defaults).
- **Contract reference:** `docs/cli-contract-schemas.md:25-34`.
- **Fix sketch:** Prefer (a) — the current shape is compact and serde-idiomatic. A one-line clarification in the schema keeps the contract honest.

### P2-3: `doctor` help omits the command's non-global purpose and status

- **Severity:** P2
- **Where:** `crates/codex1/src/cli/mod.rs:77` (`/// Report CLI health. Never crashes on missing auth or config.`) — fine as-is.
- **Observed:** `codex1 doctor --help` prints a single-line summary only. All other commands have more context in their help, but the error-surface and success-surface are compact JSON. Not a contract violation; just slightly less helpful than siblings like `close --help`.
- **Expected:** Optional: one extra sentence describing the success envelope fields (`version`, `config`, `install`, `auth`, `warnings`).
- **Contract reference:** None — cosmetic.
- **Fix sketch:** Extend the `Commands::Doctor` doc comment.

## Clean checks (no findings)

### Minimal command surface (25 verbs from `02-cli-contract.md` § Minimal Command Surface)

All verbs exist in the clap dispatch tree and resolve via `codex1 <group> <verb> --help`:

- `codex1 init` — `crates/codex1/src/cli/mod.rs:76`, `init.rs:19`.
- `codex1 status` — `cli/mod.rs:121`, `status/mod.rs:21`.
- `codex1 outcome check` / `ratify` — `outcome/mod.rs:22-35`.
- `codex1 plan choose-level` / `scaffold` / `check` / `graph` / `waves` — `plan/mod.rs:22-67`.
- `codex1 task next` / `start` / `finish` / `status` / `packet` — `task/mod.rs:22-58`.
- `codex1 review start` / `packet` / `record` / `status` — `review/mod.rs:23-81`.
- `codex1 replan check` / `record` — `replan/mod.rs:18-37`.
- `codex1 loop pause` / `resume` / `deactivate` — `loop_/mod.rs:24-47` (plus the undocumented `activate`, see P1-2).
- `codex1 close check` / `complete` — `close/mod.rs:43-73` (plus `record-review`, see P1-2).

All 25 minimal-surface commands have clap-generated `--help` text (verified by calling `--help` on each group and leaf).

### Stable JSON envelopes

- Success envelope: `{ ok: true, mission_id?, revision?, data }` per `core/envelope.rs:20-29`. Verified live on `doctor`, `init`, `status`, `hook snippet`, `outcome check` (success path), `plan choose-level`, `plan waves`, `plan graph`, `task next`, `replan check`.
- Error envelope: `{ ok: false, code, message, hint?, retryable, context? }` per `core/envelope.rs:67-78`. Verified live on `outcome check` (OUTCOME_INCOMPLETE), `close complete` (CLOSE_NOT_READY), `task start T1` (TASK_NOT_READY), `review status T99` (PLAN_INVALID).
- Compact output is available via `to_compact()`; the binary defaults to pretty-printed JSON.

### Error codes vs canonical `CliError` set

- Every `CliError` variant's `code()` string (`core/error.rs:80-106`) appears in `docs/cli-contract-schemas.md:36-57` with the one exception documented as P2-1 (`INTERNAL`).
- Every raw string code constructed under `crates/codex1/src/cli/**/*.rs` is a member of the canonical set:
  - `cli/plan/check.rs` `exit_with_validation_error` calls use only `PLAN_INVALID` / `DAG_CYCLE` / `DAG_MISSING_DEP` (per grep at lines 45, 144, 155, 165, 174, 183, 192, 201, 217, 225, 248, 256, 289, 317, 325, 339, 360, 368, 377, 386, 397, 415, 428, 438, 447, 468, 479, 491).
  - `cli/close/check.rs` `Blocker::new` codes at lines 80, 85, 93, 98, 106, 115, 121 are `OUTCOME_NOT_RATIFIED` / `PLAN_INVALID` / `REPLAN_REQUIRED` / `TASK_NOT_READY` / `REVIEW_FINDINGS_BLOCK` / `CLOSE_NOT_READY`. All canonical.
- No handler constructs a bespoke `JsonErr` with a non-canonical string; the only such path is the foundation-owned `exit_with_validation_error` using canonical strings.

### `status` and `close check` agreement (7 synthetic states)

Across seven hand-authored `STATE.json` fixtures, `codex1 status --mission <id>` and `codex1 close check --mission <id>` never disagreed on `verdict`. Both derive through `state::readiness::derive_verdict` (`crates/codex1/src/state/readiness.rs:40-66`); `status` calls it from `cli/status/project.rs:19` and `close check` from `cli/close/check.rs:47`.

| Fixture | `status.verdict` | `status.close_ready` | `close check.verdict` | `close check.ready` |
| --- | --- | --- | --- | --- |
| Fresh init (outcome unratified, plan unlocked) | `needs_user` | `false` | `needs_user` | `false` |
| `loop.paused = true`, execute phase | `continue_required` | `false` | `continue_required` | `false` |
| Plan locked + ready task, outcome ratified | `continue_required` | `false` | `continue_required` | `false` |
| Dirty review present | `blocked` | `false` | `blocked` | `false` |
| All tasks complete, mission-close review not started | `ready_for_mission_close_review` | `false` | `ready_for_mission_close_review` | `false` |
| Mission-close review passed | `mission_close_review_passed` | `true` | `mission_close_review_passed` | `true` |
| Terminal (`close.terminal_at` set) | `terminal_complete` | `false` | `terminal_complete` | `false` |

`close_ready` / `ready` are both `true` exactly when the verdict is `mission_close_review_passed`, per `state::readiness::close_ready()` (`readiness.rs:69-72`) and `ReadinessReport::ready` (`cli/close/check.rs:49`). The two fields are synonyms, backed by the same predicate.

### `stop.allow` projection consistency

- `stop.allow` is derived in `state::readiness::stop_allowed` (`readiness.rs:92-101`); `status` projects it via `cli/status/project.rs:72-99`.
- Paused state → `allow: true, reason: "paused"`. Terminal state → `allow: true, reason: "terminal"`. Active-unpaused with non-close verdict → `allow: false, reason: "active_loop"`. Active-unpaused with verdict in `{NeedsUser, MissionCloseReviewPassed}` → `allow: true, reason: "idle"`. Verified live on all three.
- Bare `codex1 status` with no mission resolvable emits a graceful `stop.allow: true, reason: "no_mission"` envelope (`cli/status/mod.rs:38-49`) so Ralph never blocks a shell with no mission present.

### Global flags

- `--mission <id>`, `--repo-root <path>`, `--json`, `--dry-run`, `--expect-revision <N>` are declared as globals in `cli/mod.rs:46-69` and appear in every `--help` output.
- Mission resolution precedence per `docs/cli-contract-schemas.md:70-76` is implemented in `core/mission::resolve_mission`; a no-mission `codex1 status` still returns a success envelope (cli/status/mod.rs:38-49).

### Build proof

`cargo build --release` in the audited tree succeeds with no warnings promoted to errors (`Finished release profile [optimized] target(s)`). No Rust source was modified by this audit.
