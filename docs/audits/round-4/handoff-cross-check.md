# Round 4 — handoff-cross-check audit

## Summary

Audited `HEAD = 703a171` against every normative claim and anti-goal across
the seven handoff docs at
`docs/codex1-rebuild-handoff/{README,00-why-and-lessons,01-product-flow,02-cli-contract,03-planning-artifacts,04-roles-models-prompts,05-build-prompt}.md`.

**0 P0, 0 P1, 0 P2, 0 P3.**

Rule-5 reminder: Round-1 REJECTed the `02-cli-contract.md:219` suggested-verdict
`complete` vs code `terminal_complete` drift (handoff-internal; handoff
frozen and `00-why-and-lessons.md:172` says `terminal_complete`). Round-2
REJECTed `ParsedPlan` `#[serde(deny_unknown_fields)]` as non-blocking P3.
Neither is re-surfaced here.

### Anti-goals

Every declared anti-goal (`README.md:82-96`, `00-why-and-lessons.md:77-90`,
`02-cli-contract.md:108-121`, `05-build-prompt.md:30-39`) is honored:

- **No `.ralph/` directory.** `find /Users/joel/codex1 -name ".ralph"
  -o -name ".ralph*"` returns zero hits. Mentions survive only in
  anti-goal prose (`.codex/skills/close/SKILL.md:112`,
  `crates/codex1/src/lib.rs`, every handoff doc, `README.md:3`).
- **No caller-identity predicates in Rust.** Grep for
  `is_parent|is_subagent|caller_type|reviewer_id|parent_id|subagent_type|caller_identity|session_owner|session_token|capability_token|authority_token`
  in `crates/codex1/src/**/*.rs` returns zero matches. Every gated
  mutation rests on artifact state
  (`plan.locked`, `close.terminal_at`, `replan.triggered`, review
  record classification), not on who called.
- **No stored waves.** `ParsedPlan` (`crates/codex1/src/cli/plan/parsed.rs:10-22`)
  has no `waves` field. `MissionState` and `PlanState`
  (`crates/codex1/src/state/schema.rs:72-86,177-194`) have no `waves`
  field. `plan scaffold` / `init` templates emit no `waves:` key.
  Wave derivation happens at `cli/plan/waves.rs` and
  `cli/status/next_action.rs` from `depends_on` + state.
- **No reviewer writeback.** Only `codex1 review record`
  (`crates/codex1/src/cli/review/record.rs`) and
  `codex1 close record-review` (`crates/codex1/src/cli/close/record_review.rs`)
  transition review state; both are main-thread-invoked. Skill prompts
  at `.codex/skills/review-loop/SKILL.md:69` make this explicit:
  "Reviewer writeback is forbidden."
- **No per-task lane state beyond status/proof.** `TaskRecord`
  (`crates/codex1/src/state/schema.rs:99-111`) carries only
  `id/status/started_at/finished_at/proof_path/superseded_by` — no
  lane/lock/reviewer identity fields. `ReviewRecord:131-141` carries
  `reviewers: Vec<String>` as a display csv (see `02-cli-contract.md:417`:
  `--reviewers code-reviewer,intent-reviewer`), not as an authority token;
  `boundary_revision` is late-output classification, not identity.

Adjacent non-goals held:

- **No wrapper runtime.** Grep for `Command::new` / `tokio::spawn` /
  `daemon` / `spawn_process` in `crates/codex1/src` returns zero matches.
- **Ralph is status-only.** `scripts/ralph-stop-hook.sh:37` runs exactly
  `codex1 status --json` and dispatches on `.data.stop.allow`. Matches
  `01-product-flow.md:232-243`.

### Structural invariants

- **6 skills + SKILL.md + `agents/openai.yaml`.** All six required
  skills (`autopilot`, `clarify`, `close`, `execute`, `plan`,
  `review-loop`) exist at `.codex/skills/<name>/`. Every one has both
  `SKILL.md` and `agents/openai.yaml`. Matches `01-product-flow.md:7-17`.
- **Mission layout under `PLANS/<mission-id>/`.** `cli/init.rs:44-46`
  creates mission scaffold via `state::init_write` +
  `write_outcome_template` + `write_plan_template`. Populates
  `OUTCOME.md`, `PLAN.yaml`, `STATE.json`, `EVENTS.jsonl`, `specs/`,
  `reviews/` (`state/mod.rs:166-177`). `CLOSEOUT.md` written by
  `cli/close/complete.rs:81-82`. Matches `03-planning-artifacts.md:9-26`.
- **Atomic-write protocol.** `state/fs_atomic.rs:22-37` follows
  tempfile-in-dir → `sync_data` → `persist` (rename) → parent-dir
  `sync_all`. Matches `02-cli-contract.md:385-391`. Wrapped under an
  exclusive fs2 lock inside `state::mutate` (`state/mod.rs:101-152`)
  with the documented "append EVENTS.jsonl FIRST, then persist
  STATE.json" ordering preserved (rationale inline at
  `state/mod.rs:126-141`).
- **Mission resolution precedence.** `core/mission.rs:28-51` implements
  `--mission + --repo-root` → `--mission` alone (CWD) → `--repo-root`
  alone (`discover_single_mission`) → CWD walk-up
  (`walk_up_for_mission`). Matches the docstring precedence at
  `core/mission.rs:3-6` and the handoff expectation at
  `02-cli-contract.md:39-44` ("Supports `--mission`… `--repo-root`…
  avoid hidden active-mission resolution").
- **Verdict derivation ordering.** `state/readiness.rs:40-66`:
  `TerminalComplete` → `NeedsUser(!outcome.ratified)` →
  `NeedsUser(!plan.locked)` → `Blocked(replan.triggered)` →
  `Blocked(has_blocking_dirty)` → mission-close family
  (`ReadyForMissionCloseReview` / `MissionCloseReviewOpen` /
  `MissionCloseReviewPassed`) when `tasks_complete` → `ContinueRequired`.
  Consumed identically by `status`
  (`cli/status/project.rs:19`) and `close check`
  (`cli/close/check.rs:47-49`) through
  `ReadinessReport::from_state`, so the two endpoints cannot diverge
  — the handoff invariant at `02-cli-contract.md:208` ("status and close
  check must share readiness logic"). Suggested verdict vocabulary at
  `02-cli-contract.md:212-221` and `00-why-and-lessons.md:166-176`
  matches `Verdict::as_str` at `readiness.rs:22-34`.
- **EVENTS.jsonl append-only + monotonic seq.** `state/events.rs:41-47`
  opens with `OpenOptions::new().create(true).append(true).open(path)`.
  `seq = state.events_cursor`, bumped exactly once per successful
  mutation (`state/mod.rs:125`) and never rewound. No code path
  writes historical events.
- **Hard-planning evidence enforced.** `cli/plan/check.rs:273-309`
  rejects `effective: hard` plans unless `planning_process.evidence`
  is non-empty AND at least one entry has `kind ∈ {explorer, advisor,
  plan_review}` (`HARD_EVIDENCE_KINDS` in
  `cli/plan/parsed.rs:112`). Matches `02-cli-contract.md:349-350`
  and `03-planning-artifacts.md:280-287`.

All 14 handoff-suggested error codes (`02-cli-contract.md:520-534`) are
present as `CliError` variants with stable `code()` strings:
`OUTCOME_INCOMPLETE`, `OUTCOME_NOT_RATIFIED`, `PLAN_INVALID`,
`DAG_CYCLE`, `DAG_MISSING_DEP`, `TASK_NOT_READY`, `PROOF_MISSING`,
`REVIEW_FINDINGS_BLOCK`, `REPLAN_REQUIRED`, `CLOSE_NOT_READY`,
`STATE_CORRUPT`, `REVISION_CONFLICT`, `STALE_REVIEW_RECORD`,
`TERMINAL_ALREADY_COMPLETE` (`core/error.rs:83-96`).

Skill model matrix matches handoff (`04-roles-models-prompts.md:21-22,
163-167`): `.codex/skills/review-loop/SKILL.md:57-63` lists
`code_bug_correctness → gpt-5.3-codex high`,
`local_spec_intent → gpt-5.4 high`,
`integration_intent → gpt-5.4 high`,
`plan_quality → gpt-5.4 high|xhigh`,
`mission_close → gpt-5.4 high (2 lanes)`. No drift.

## Round-4 spot checks

### Spot check 1 — OUTCOME.md frontmatter preservation across `rewrite_status_to_ratified`

Round-3 e2e P1-1 fix landed at `cli/outcome/ratify.rs:105-154`. The
rewrite now emits `---\n` unconditionally for the closing fence, then
pastes the body verbatim; byte-stable across both "blank line between
fence and heading" and "fence directly followed by heading" authoring
styles. Rationale documented inline at `cli/outcome/ratify.rs:140-151`.

Handoff support: `03-planning-artifacts.md:60` states: "Recommended
`OUTCOME.md` shape can be YAML frontmatter plus readable markdown, or
pure YAML." No exact byte-format is specified; the contract is
"frontmatter fields the CLI can check." The round-3 fix respects this
— it preserves every frontmatter byte other than the first `status:`
line, and it preserves every body byte. Required fields
(`03-planning-artifacts.md:64-105`) are still checkable via
`validate_outcome`. Tests at `crates/codex1/tests/outcome.rs:391-471`
(no-blank-line repro) and `:482-536` (two-ratify idempotence) confirm
the rewritten file re-parses through `outcome check`. **Honored.**

### Spot check 2 — `review-loop` skill dispatch accepts both mission-close entry verdicts

Round-3 skills P1-1 fix landed at
`.codex/skills/review-loop/SKILL.md:39-43`. The mission-close workflow
now accepts both:

- `data.verdict == ready_for_mission_close_review` — first round.
- `data.verdict == mission_close_review_open` — re-entry after a dirty
  `close record-review` flipped `close.review_state = Open`
  (`cli/close/record_review.rs:205`).

Handoff support: both verdicts are listed as valid states in the
suggested verdict vocabulary at `02-cli-contract.md:216-217` and
`00-why-and-lessons.md:169-170`. The autopilot state machine at
`.codex/skills/autopilot/references/autopilot-state-machine.md:20-21`
routes both to `$review-loop` (mission-close mode). Close gate at
`cli/close/record_review.rs:67-78` accepts entries from both
`Verdict::ReadyForMissionCloseReview` and
`Verdict::MissionCloseReviewOpen` precisely because the dirty mission-
close record path (`record_review.rs:205`) leaves review_state at
`Open` pending another round. Without the skill accepting both, a
dirty mission-close review would leave the mission stuck: the CLI
would route `$review-loop` in, but the skill would refuse the handoff.
**Honored.**

## Round-1/2/3 fix verifications (no regressions)

All prior fixes remain intact at HEAD:

- `state::require_plan_locked` pre-mutate calls
  (`cli/task/start.rs:22`, `cli/task/finish.rs:21`,
  `cli/review/start.rs:41`, `cli/review/record.rs:77`) and in-closure
  re-checks (`cli/task/start.rs:111`, `cli/task/finish.rs:112`,
  `cli/review/start.rs:95`, `cli/review/record.rs:185-187`) — present.
- EVENTS-before-STATE ordering in `state::mutate`
  (`state/mod.rs:142-145`) — present with rationale.
- Parent-dir `sync_all` after `persist` (`state/fs_atomic.rs:33-35`) —
  present.
- `outcome ratify` state-first-then-OUTCOME.md atomicity
  (`cli/outcome/ratify.rs:60-75`) — preserved.
- `--expect-revision` enforcement on every short-circuit:
  `task/start.rs`, `task/finish.rs`, `plan/check.rs:78`,
  `plan/choose_level.rs`, `review/start.rs:95` (dry-run via
  `check_expected_revision` — round-2 correctness P2-1),
  `review/record.rs`, `close/complete.rs:53`,
  `close/record_review.rs:95,154`, `outcome/ratify.rs:34`, `loop_/mod.rs`.
- Six-consecutive-dirty replan threshold `= 6`
  (`cli/review/record.rs:29`, `cli/close/record_review.rs:26`).
- `cli/plan/check.rs:133-134` clears `state.replan.triggered` +
  `triggered_reason` at relock (round-2 e2e P0-1). Rationale inline at
  `check.rs:125-132`.
- `cli/task/next.rs:26-53` short-circuits on `!plan.locked` and
  `replan.triggered` mirroring `cli/status/project.rs::derive_next_action`.
- `.codex/skills/plan/SKILL.md:190` and
  `.codex/skills/plan/references/dag-quality.md:46` replan-record
  snippets carry `--reason`.
- `.codex/skills/plan/SKILL.md:20` says `outcome_ratified`;
  `.codex/skills/execute/SKILL.md:16` says `plan_locked` — matches
  the flat status projection at `cli/status/project.rs:72-73`.
- `.codex/skills/review-loop/SKILL.md:39-43` and
  `.codex/skills/review-loop/references/reviewer-profiles.md` preserve
  the expanded dispatch + aligned reviewer matrix.
- `cli/outcome/ratify.rs:140-151` preserves the closing-fence newline
  unconditionally (round-3 e2e P1-1).
- Direct `STATE_CORRUPT` integration test at
  `crates/codex1/tests/foundation.rs::state_corrupt_envelope_on_invalid_state_json`
  covers the `serde_json::from_str` branch at `state/mod.rs:84`
  (round-3 test-adequacy P2-1).
- `full_mission_close_after_replan_reaches_terminal` now asserts
  CLOSEOUT.md presence + body shape (round-3 test-adequacy P2-2).

## P0

None.

## P1

None.

## P2

None.

## P3

None.
