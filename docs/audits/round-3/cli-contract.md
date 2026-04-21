# Round 3 — cli-contract audit

**Commit audited:** ccff0b2
**Lens:** cli-contract
**Reviewer:** Opus 4.7

## Summary

Walked every command in `docs/codex1-rebuild-handoff/02-cli-contract.md` against `crates/codex1/src/cli/**/*.rs`, exercised each with `--json` against a temp mission for a full clarify → plan → execute → review → mission-close → terminal cycle, and verified `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` both pass clean. No new P0/P1/P2 contract violations found; previously-flagged drifts (`data`-wrapping envelope, `complete` vs `terminal_complete`) remain as accepted by round-1/round-2 decisions.

## P0

None.

## P1

None.

## P2

None.

## P3 (non-blocking)

### Verification checklist (all passing)

- **Command surface parity.** Every command in the minimal surface (`02-cli-contract.md:59-94`) resolves in `Commands` enum (`crates/codex1/src/cli/mod.rs:73-125`): `init`, `status`, `outcome check/ratify`, `plan choose-level/scaffold/check/graph/waves`, `task next/start/finish/status/packet`, `review start/packet/record/status`, `replan record/check`, `loop pause/resume/deactivate/activate`, `close check/complete/record-review`. Plus documented utility extras `doctor` and `hook snippet` (not in the handoff's minimal surface, but documented on the enum variants).
- **Global-flag surface on every command.** Spot-checked `--mission`, `--repo-root`, `--json`, `--dry-run`, `--expect-revision` on `status`, `plan check`, `task packet`, `close record-review`, `loop activate` — present via `#[arg(... global = true)]` on `Cli` at `mod.rs:44-66`.
- **Error-code set.** All 14 suggested canonical codes (`02-cli-contract.md:518-535`) map to `CliError` variants in `crates/codex1/src/core/error.rs:82-103`: `OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING`, `REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`, `CLOSE_NOT_READY`, `STATE_CORRUPT`, `REVISION_CONFLICT`, `STALE_REVIEW_RECORD`, `TERMINAL_ALREADY_COMPLETE`. Reserved extras (`CONFIG_MISSING`, `NOT_IMPLEMENTED`, `MISSION_NOT_FOUND`, `PARSE_ERROR`) preserved per round-1 decision. Live exercises:
  - `plan check` on a plan with fill markers → `PLAN_INVALID` with `marker_preview` context.
  - `plan check` on cyclic DAG → `DAG_CYCLE` with `cycle_edges` context.
  - `plan check` on missing-dep → `DAG_MISSING_DEP` with `missing_dep` context.
  - `plan check` on duplicate ids → `PLAN_INVALID` with `duplicate_ids`.
  - `plan check` on invalid id format (e.g. `R1`) → `PLAN_INVALID` with `invalid_id`.
  - `outcome ratify` with fill markers → `OUTCOME_INCOMPLETE` with 23-entry `placeholders` list.
  - `loop activate --expect-revision 99` on fresh state → `REVISION_CONFLICT` `{expected:99, actual:0}` + hint.
  - `plan choose-level --level 1 --json` (pre-ratify) → `OUTCOME_NOT_RATIFIED`.
  - `close complete` on terminal mission → `TERMINAL_ALREADY_COMPLETE` with `closed_at` context.
- **Verdict values.** 7/8 handoff-suggested values match `Verdict::as_str()` (`crates/codex1/src/state/readiness.rs:22-35`). The remaining drift — `complete` in the handoff vs `terminal_complete` in code — was REJECTED in round-1 decisions (`00-why-and-lessons.md:173` is canonical; handoff frozen per rule 5).
- **`plan choose-level` escalation payload.** Matches handoff example at lines 292-316:
  - Non-escalated (`--level medium`): `escalation_required: false`, no `escalation_reason`.
  - Escalated (`--level light --escalate "hooks modified"`): `escalation_required: true`, `escalation_reason` present, `effective_level: hard`, `next_action.args` rewritten to `["codex1","plan","scaffold","--level","hard"]`.
  - `--escalate` on `--level hard`: correctly drops the reason (no phantom escalation), per `crates/codex1/src/cli/plan/choose_level.rs:40-46`. This is the round-1 P2 F1 fix, still intact.
- **Numeric-alias / canonical-value parity.** `choose_level::parse_level` accepts `1/light`, `2/medium`, `3/hard` (matches `02-cli-contract.md:260-266`); stored values are always the verbs (`light|medium|hard`). `low`/`high` rejected with `PARSE_ERROR`.
- **Review-freshness categories.** All four categories at `02-cli-contract.md:445-450` emitted from `crates/codex1/src/cli/review/classify.rs:71-77`: `accepted_current`, `late_same_boundary`, `stale_superseded`, `contaminated_after_terminal`.
- **Review profile set.** `code_bug_correctness`, `local_spec_intent`, `integration_intent`, `plan_quality`, `mission_close` all recognized by the reviewer-profile matrix (handoff lines 430-435); `review packet` emits the profile string.
- **Replan reason codes.** `crates/codex1/src/cli/replan/triggers.rs:13-19` list: `six_dirty`, `scope_change`, `architecture_shift`, `risk_discovered`, `user_request`. Enforced at `replan record --reason` (round-1 `tests/replan.rs::record_rejects_unknown_reason`).
- **`close record-review` is the single Open → Passed transition.** Handoff lines 100-104 require this; `crates/codex1/src/cli/close/record_review.rs` is the only path that sets `close.review_state = Passed`. Round-1 `tests/close.rs::record_review_open_then_clean_transitions_to_passed` covers it.
- **`close check` vs `status` agreement.** Handoff line 208 requires agreement; both consume the same `readiness::derive_verdict`/`close_ready` predicates (`crates/codex1/src/state/readiness.rs`). Verified end-to-end against a terminal mission: `close check` returns `verdict: terminal_complete, ready: false` and `status` returns `verdict: terminal_complete, close_ready: false, stop.reason: "terminal"`.
- **Envelope shape.** Error envelope omits `hint`/`context` when empty via `#[serde(skip_serializing_if = ...)]` in `crates/codex1/src/core/envelope.rs:73-78`. Success envelope omits `mission_id`/`revision` when absent. Stable across commands.
- **Initial STATE.json from `init`.** Contains the handoff-required subset (`02-cli-contract.md:139-147`): `mission_id`, `loop {active:false, paused:false, mode:"none"}`, `tasks:{}`, `reviews:{}`, `phase:"clarify"`. Extra fields (`revision`, `schema_version`, `outcome`, `plan`, `replan`, `close`, `events_cursor`) are additive; the handoff shows a minimum, not an upper bound.
- **`--proof` path semantics.** `task finish T1 --proof specs/T1/PROOF.md` resolves against `paths.mission_dir` (`crates/codex1/src/cli/task/finish.rs:33-42`) — matches handoff example at line 376. Absolute paths taken verbatim.
- **`plan graph --format mermaid`** emits a valid flowchart with classDef styling and edges (`crates/codex1/src/cli/plan/graph.rs:120-147`). `--format dot` and `--format json` also wired.
- **`plan waves --json`** derives from DAG at call time; no `waves` field stored in STATE (confirmed by inspection of `state/schema.rs:178-194`). Matches anti-goal at `02-cli-contract.md:118`.
- **Build artifact.** `make install-local` succeeds, installs to `/Users/joel/.local/bin/codex1`, `codex1 --version` returns `0.1.0` from any cwd, `codex1 doctor --json` reports `install.codex1_on_path` and `home_local_bin_writable: true`.
- **fmt / clippy.** `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` both clean against commit ccff0b2. `cargo test --release` fully green (all 17 test binaries pass).

### Accepted deviations from handoff examples (out of scope)

- **Status `data`-wrapping.** Handoff example at `02-cli-contract.md:179-206` shows `phase`, `loop`, `next_action`, etc. at the envelope's top level; implementation wraps them in `data`. Two prior rounds have left this as accepted harness-envelope convention (`core/envelope.rs:22-29`); re-flagging would duplicate decisions already made.
- **`Commands::Status` unit variant has no explicit sub-args struct.** Global flags cover the required flag surface; this is a style detail.
- **`hook` and `doctor` commands not in the minimal surface.** `doctor` serves the verification-bar checklist (health probe for install); `hook snippet` is a Ralph wiring helper. Neither is a contract violation — they are additive utilities, and the handoff rule is "do not add more *core* commands until the minimal set is excellent".
